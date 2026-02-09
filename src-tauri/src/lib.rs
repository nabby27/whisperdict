mod app_state;
mod audio;
mod child_transcribe;
mod config;
mod hotkeys;
mod models;
mod paste;
mod recording;
mod transcription;
mod tray;
mod wayland_hotkeys;

use app_state::{AppState, StatusResponse};
use serde::Serialize;
use tauri::{image::Image, AppHandle, Manager, State};
use tauri_plugin_updater::UpdaterExt;

const UPDATER_ENDPOINT: Option<&str> = option_env!("WHISPERDICT_UPDATER_ENDPOINT");
const UPDATER_PUBKEY: Option<&str> = option_env!("WHISPERDICT_UPDATER_PUBKEY");

#[derive(Serialize)]
struct ModelState {
    id: String,
    title: String,
    size_mb: u32,
    installed: bool,
    partial: bool,
    active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigState {
    shortcut: String,
    active_model_id: String,
    language: String,
    free_transcriptions_left: u32,
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Result<ConfigState, String> {
    let config = state.get_settings().map_err(|e| e.to_string())?;
    Ok(ConfigState {
        shortcut: config.shortcut,
        active_model_id: config.active_model,
        language: config.language,
        free_transcriptions_left: config.free_transcriptions_left,
    })
}

#[tauri::command]
fn set_shortcut(state: State<'_, AppState>, shortcut: String) -> Result<(), String> {
    state.set_shortcut(&shortcut).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    state.set_language(&language).map_err(|e| e.to_string())
}

async fn check_for_updates(app: AppHandle) {
    let Some(endpoint) = UPDATER_ENDPOINT else {
        return;
    };
    let Some(pubkey) = UPDATER_PUBKEY else {
        return;
    };

    let endpoint = match endpoint.parse() {
        Ok(endpoint) => endpoint,
        Err(_) => return,
    };

    let updater = match app
        .updater_builder()
        .pubkey(pubkey)
        .endpoints(vec![endpoint])
    {
        Ok(builder) => builder,
        Err(_) => return,
    };

    let updater = match updater.build() {
        Ok(updater) => updater,
        Err(_) => return,
    };

    let update = match updater.check().await {
        Ok(update) => update,
        Err(_) => return,
    };

    if let Some(update) = update {
        if update.download_and_install(|_, _| {}, || {}).await.is_ok() {
            app.restart();
        }
    }
}


#[tauri::command]
async fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelState>, String> {
    let response = state.list_models().await.map_err(|e| e.to_string())?;
    Ok(response
        .models
        .into_iter()
        .map(|model| ModelState {
            id: model.id.clone(),
            title: model.id[..1].to_uppercase() + &model.id[1..],
            size_mb: model.size_mb,
            installed: model.installed,
            partial: model.partial,
            active: model.id == response.active_model,
        })
        .collect())
}

#[tauri::command]
async fn download_model(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    state
        .download_model(&app, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_model(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.delete_model(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn set_active_model(state: State<'_, AppState>, app: AppHandle, id: String) -> Result<(), String> {
    state.set_active_model(&id).map_err(|e| e.to_string())?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<AppState>();
        let _ = state.preload_transcribe_server(&handle).await;
    });
    Ok(())
}

#[tauri::command]
async fn toggle_recording(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let recording = state.status().recording;
    if recording {
        state.stop_recording(&app).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        state.start_recording(&app).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Result<StatusResponse, String> {
    Ok(state.status())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    if let (Some(_), Some(pubkey)) = (UPDATER_ENDPOINT, UPDATER_PUBKEY) {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().pubkey(pubkey).build());
    }

    builder
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let state = AppState::new(&app.handle()).map_err(|e| e.to_string())?;
            state.tray.init(&app.handle());
            let hotkey = state.hotkey.clone();
            let handle = app.handle().clone();
            let _ = hotkeys::start_listener(handle, hotkey);
            app.manage(state);
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) = Image::from_bytes(include_bytes!("../icons-app/32x32.png")) {
                    let _ = window.set_icon(icon);
                }
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                let _ = state.preload_transcribe_server(&handle).await;
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                check_for_updates(handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_shortcut,
            set_language,
            list_models,
            download_model,
            delete_model,
            set_active_model,
            toggle_recording,
            get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running Whisperdict");
}

pub fn run_child() -> anyhow::Result<bool> {
    child_transcribe::run_if_child()
}
