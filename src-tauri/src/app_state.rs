use crate::audio::resample_to_16k;
use crate::config::{load_config, save_config, AppConfig};
use crate::hotkeys::Hotkey;
use crate::models;
use crate::paste::paste_text;
use crate::recording::RecorderWorker;
use crate::tray::{TrayController, TrayMode};
use crate::wayland_hotkeys::WaylandHotkeys;
use anyhow::{Context, Result};
use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::{env, fs, path::PathBuf, time::SystemTime};
use tauri::{AppHandle, Emitter};
use tokio::task;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub tray: TrayController,
    pub hotkey: Arc<Mutex<Hotkey>>,
    pub recorder: RecorderWorker,
    pub wayland_hotkeys: Option<WaylandHotkeys>,
    transcribe: Arc<Mutex<Option<TranscribeServer>>>,
}

#[derive(Serialize)]
pub struct ModelListResponse {
    pub models: Vec<models::ModelStatus>,
    pub active_model: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub recording: bool,
}

#[derive(Serialize, Clone)]
pub struct ModelProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionEvent {
    pub text: String,
    pub model_id: String,
    pub duration_ms: u64,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let mut config = load_config().unwrap_or_default();
        let installed = models::list_models().unwrap_or_default();
        let installed_ids: Vec<String> = installed
            .into_iter()
            .filter(|m| m.installed)
            .map(|m| m.id)
            .collect();
        if !installed_ids.contains(&config.active_model) {
            if installed_ids.contains(&config.preferred_model) {
                config.active_model = config.preferred_model.clone();
            } else {
                config.active_model = "base".to_string();
            }
        }
        let hotkey = Hotkey::parse(&config.shortcut).unwrap_or(Hotkey {
            ctrl: true,
            alt: true,
            shift: false,
            key: rdev::Key::Space,
        });
        let wayland_hotkeys = WaylandHotkeys::start(app.clone(), config.shortcut.clone());
        let state = Self {
            config: Arc::new(Mutex::new(config)),
            tray: TrayController::new(),
            hotkey: Arc::new(Mutex::new(hotkey)),
            recorder: RecorderWorker::new(),
            wayland_hotkeys,
            transcribe: Arc::new(Mutex::new(None)),
        };
        state.tray.start_animation();
        state.tray.set_mode(TrayMode::Idle);
        Ok(state)
    }

    pub async fn list_models(&self) -> Result<ModelListResponse> {
        let models = models::list_models()?;
        let config = self.config.lock().unwrap().clone();
        Ok(ModelListResponse {
            models,
            active_model: config.active_model,
        })
    }

    pub async fn download_model(&self, app: &AppHandle, model_id: &str) -> Result<()> {
        let app_handle = app.clone();
        let model_id_owned = model_id.to_string();
        let start_event = ModelProgress {
            model_id: model_id_owned.clone(),
            downloaded: 0,
            total: None,
            done: false,
            error: None,
        };
        let _ = app.emit("models:progress", start_event);
        let result = models::download_model_with_progress(model_id, move |downloaded, total| {
            let event = ModelProgress {
                model_id: model_id_owned.clone(),
                downloaded,
                total,
                done: false,
                error: None,
            };
            let _ = app_handle.emit("models:progress", event);
        })
        .await;

        match result {
            Ok(_) => {
                let event = ModelProgress {
                    model_id: model_id.to_string(),
                    downloaded: 0,
                    total: None,
                    done: true,
                    error: None,
                };
                let _ = app.emit("models:progress", event);
                Ok(())
            }
            Err(err) => {
                let event = ModelProgress {
                    model_id: model_id.to_string(),
                    downloaded: 0,
                    total: None,
                    done: true,
                    error: Some(err.to_string()),
                };
                let _ = app.emit("models:progress", event);
                Err(err)
            }
        }
    }

    pub async fn delete_model(&self, model_id: &str) -> Result<()> {
        models::delete_model(model_id)?;
        let installed = models::list_models()?;
        let installed_ids: Vec<String> = installed
            .into_iter()
            .filter(|m| m.installed)
            .map(|m| m.id)
            .collect();
        let mut config = self.config.lock().unwrap();
        if config.active_model == model_id {
            if installed_ids.contains(&config.preferred_model) {
                config.active_model = config.preferred_model.clone();
            } else if installed_ids.contains(&"base".to_string()) {
                config.active_model = "base".to_string();
            } else {
                config.active_model = "none".to_string();
            }
            save_config(&config)?;
        }
        Ok(())
    }

    pub fn set_active_model(&self, model_id: &str) -> Result<()> {
        let mut config = self.config.lock().unwrap();
        config.active_model = model_id.to_string();
        config.preferred_model = model_id.to_string();
        save_config(&config)?;
        Ok(())
    }

    pub fn get_settings(&self) -> Result<AppConfig> {
        Ok(self.config.lock().unwrap().clone())
    }

    pub fn set_language(&self, language: &str) -> Result<()> {
        let mut config = self.config.lock().unwrap();
        config.language = language.to_string();
        save_config(&config)?;
        Ok(())
    }

    fn decrement_transcriptions(&self) -> Result<()> {
        let mut config = self.config.lock().unwrap();
        if config.free_transcriptions_left > 0 {
            config.free_transcriptions_left -= 1;
            save_config(&config)?;
        }
        Ok(())
    }

    pub async fn preload_transcribe_server(&self, app: &AppHandle) -> Result<()> {
        let config = self.config.lock().unwrap().clone();
        let model_id = config.active_model.clone();
        if model_id == "none" {
            return Ok(());
        }
        let model_path = models::model_path(&model_id)?;
        if !models::model_is_valid(&model_id)? {
            self.download_model(app, &model_id).await?;
        }
        let model_path_str = model_path.to_string_lossy().to_string();
        let mut guard = self.transcribe.lock().unwrap();
        let needs_restart = guard
            .as_ref()
            .map(|s| s.model_id != model_id)
            .unwrap_or(true);
        if needs_restart {
            *guard = Some(spawn_server(&model_id, &model_path_str)?);
        }
        Ok(())
    }

    pub fn set_shortcut(&self, shortcut: &str) -> Result<()> {
        let mut config = self.config.lock().unwrap();
        config.shortcut = shortcut.to_string();
        save_config(&config)?;
        if let Some(parsed) = Hotkey::parse(shortcut) {
            let mut hk = self.hotkey.lock().unwrap();
            *hk = parsed;
        }
        if let Some(wayland) = &self.wayland_hotkeys {
            wayland.update(shortcut.to_string());
        }
        Ok(())
    }

    pub fn status(&self) -> StatusResponse {
        let recording = self.recorder.is_recording();
        StatusResponse { recording }
    }

    pub fn start_recording(&self, app: &AppHandle) -> Result<()> {
        if self.recorder.is_recording() {
            return Ok(());
        }
        self.recorder.start().context("start recorder")?;
        self.tray.set_mode(TrayMode::Recording);
        let _ = app.emit(
            "status:changed",
            serde_json::json!({ "status": "recording", "message": null }),
        );
        Ok(())
    }

    pub async fn stop_recording(&self, app: &AppHandle) -> Result<String> {
        if !self.recorder.is_recording() {
            return Ok(String::new());
        }
        self.tray.set_mode(TrayMode::Processing);
        let _ = app.emit(
            "status:changed",
            serde_json::json!({ "status": "processing", "message": null }),
        );
        let audio = resample_to_16k(self.recorder.stop()?);
        if audio.samples.is_empty() {
            self.tray.set_mode(TrayMode::Idle);
            return Ok(String::new());
        }
        let config = self.config.lock().unwrap().clone();
        let model_id = config.active_model.clone();
        let model_path = models::model_path(&model_id)?;
        if !models::model_is_valid(&model_id)? {
            self.download_model(app, &model_id).await?;
        }
        let wav_path = write_temp_wav(&audio.samples)?;
        let model_path_str = model_path.to_string_lossy().to_string();
        let wav_path_str = wav_path.to_string_lossy().to_string();
        let server = self.transcribe.clone();
        let model_id_clone = model_id.clone();
        let start = std::time::Instant::now();
        let language = config.language.clone();
        let text_result = task::spawn_blocking(move || {
            transcribe_with_server(
                server,
                &model_id_clone,
                &model_path_str,
                &wav_path_str,
                &language,
            )
        })
        .await
        .context("transcribe task")?;
        let text = match text_result {
            Ok(text) => text,
            Err(err) => {
                self.tray.set_mode(TrayMode::Error);
                let _ = app.emit(
                    "status:changed",
                    serde_json::json!({ "status": "error", "message": err.to_string() }),
                );
                return Err(err);
            }
        };
        let _ = fs::remove_file(&wav_path);
        if !text.is_empty() {
            let _ = paste_text(&text);
            let _ = self.decrement_transcriptions();
        }
        let _ = app.emit(
            "transcription:result",
            TranscriptionEvent {
                text: text.clone(),
                model_id: model_id.clone(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        );
        self.tray.set_mode(TrayMode::Idle);
        let _ = app.emit(
            "status:changed",
            serde_json::json!({ "status": "idle", "message": null }),
        );
        Ok(text)
    }
}

fn write_temp_wav(samples: &[f32]) -> Result<PathBuf> {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    path.push(format!("ECO-{}.wav", stamp));

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).context("create wav")?;
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let value = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(value).context("write wav sample")?;
    }
    writer.finalize().context("finalize wav")?;
    Ok(path)
}

struct TranscribeServer {
    model_id: String,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

fn transcribe_with_server(
    server: Arc<Mutex<Option<TranscribeServer>>>,
    model_id: &str,
    model_path: &str,
    wav_path: &str,
    language: &str,
) -> Result<String> {
    let mut guard = server.lock().unwrap();
    let needs_restart = guard
        .as_ref()
        .map(|s| s.model_id != model_id)
        .unwrap_or(true);

    if needs_restart {
        *guard = Some(spawn_server(model_id, model_path)?);
    }

    let srv = guard.as_mut().context("missing server")?;
    writeln!(srv.stdin, "{}\t{}", language, wav_path).context("write wav path")?;
    srv.stdin.flush().context("flush stdin")?;
    let mut line = String::new();
    let read = srv.stdout.read_line(&mut line).context("read child")?;
    if read == 0 || line.trim().is_empty() {
        *guard = Some(spawn_server(model_id, model_path)?);
        let srv = guard.as_mut().context("missing server")?;
        writeln!(srv.stdin, "{}\t{}", language, wav_path)
            .context("write wav path retry")?;
        srv.stdin.flush().context("flush stdin retry")?;
        line.clear();
        srv.stdout.read_line(&mut line).context("read child retry")?;
    }
    Ok(line.trim().to_string())
}

fn spawn_server(model_id: &str, model_path: &str) -> Result<TranscribeServer> {
    let exe = env::current_exe().context("current exe")?;
    let mut child = Command::new(exe)
        .arg("--transcribe-server")
        .arg("--model")
        .arg(model_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("spawn server")?;

    let stdin = child.stdin.take().context("child stdin")?;
    let stdout = child.stdout.take().context("child stdout")?;
    Ok(TranscribeServer {
        model_id: model_id.to_string(),
        stdin,
        stdout: BufReader::new(stdout),
    })
}
