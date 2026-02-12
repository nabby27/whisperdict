mod app_state;
mod audio;
mod child_transcribe;
mod command_errors;
mod config;
mod global_config;
mod hotkeys;
mod licensing;
mod models;
mod paste;
mod recording;
mod transcription;
mod tray;
mod wayland_hotkeys;

use app_state::{AppState, StatusResponse};
use serde::{Deserialize, Serialize};
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
    total_transcriptions_count: u64,
    entitlement: String,
    license_status: String,
    license_file_path: Option<String>,
    license_last_validated_at: Option<u64>,
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Result<ConfigState, String> {
    let config = state.get_settings().map_err(command_errors::map_error)?;
    Ok(ConfigState {
        shortcut: config.shortcut,
        active_model_id: config.active_model,
        language: config.language,
        free_transcriptions_left: config.free_transcriptions_left,
        total_transcriptions_count: config.total_transcriptions_count,
        entitlement: config.entitlement,
        license_status: config.license_status,
        license_file_path: config.license_file_path,
        license_last_validated_at: config.license_last_validated_at,
    })
}

#[tauri::command]
fn set_shortcut(state: State<'_, AppState>, shortcut: String) -> Result<(), String> {
    state
        .set_shortcut(&shortcut)
        .map_err(command_errors::map_error)
}

#[tauri::command]
fn set_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    state
        .set_language(&language)
        .map_err(command_errors::map_error)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutSession {
    checkout_url: String,
    checkout_session_id: String,
}

#[derive(Deserialize)]
struct CheckoutSessionPayload {
    #[serde(alias = "checkoutUrl", alias = "checkout_url", alias = "url")]
    checkout_url: Option<String>,
    #[serde(
        alias = "checkoutSessionId",
        alias = "checkout_session_id",
        alias = "sessionId",
        alias = "session_id"
    )]
    checkout_session_id: Option<String>,
}

fn get_device_mac_address() -> String {
    mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|address| address.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[tauri::command]
async fn create_checkout_session() -> Result<CheckoutSession, String> {
    let endpoint = global_config::checkout_endpoint()
        .ok_or_else(|| "Checkout endpoint is not configured".to_string())?;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?;

    let mut request = client.post(endpoint).json(&serde_json::json!({
        "source": "whisperdict-desktop",
        "platform": std::env::consts::OS,
        "macAddress": get_device_mac_address(),
    }));

    if let Some(token) = global_config::checkout_bearer_token() {
        request = request.bearer_auth(token);
    }

    let response = request
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;

    let payload: CheckoutSessionPayload =
        response.json().await.map_err(|error| error.to_string())?;
    let checkout_url = payload
        .checkout_url
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Checkout URL is missing from checkout response".to_string())?;
    let checkout_session_id = payload
        .checkout_session_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(CheckoutSession {
        checkout_url,
        checkout_session_id,
    })
}

#[tauri::command]
fn import_license_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<licensing::LicenseImportResponse, String> {
    state
        .import_license_file(&path)
        .map_err(command_errors::map_error)
}

#[tauri::command]
fn get_license_state(state: State<'_, AppState>) -> Result<licensing::LicenseState, String> {
    state.get_license_state().map_err(command_errors::map_error)
}

#[tauri::command]
fn remove_license(state: State<'_, AppState>) -> Result<(), String> {
    state.remove_license().map_err(command_errors::map_error)
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
    let response = state
        .list_models()
        .await
        .map_err(command_errors::map_error)?;
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
        .map_err(command_errors::map_error)
}

#[tauri::command]
async fn delete_model(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .delete_model(&id)
        .await
        .map_err(command_errors::map_error)
}

#[tauri::command]
fn set_active_model(state: State<'_, AppState>, app: AppHandle, id: String) -> Result<(), String> {
    state
        .set_active_model(&id)
        .map_err(command_errors::map_error)?;
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
        state
            .stop_recording(&app)
            .await
            .map_err(command_errors::map_error)?;
        Ok(())
    } else {
        state
            .start_recording(&app)
            .map_err(command_errors::map_error)
    }
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Result<StatusResponse, String> {
    Ok(state.status())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

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
            let state = AppState::new(&app.handle()).map_err(command_errors::map_error)?;
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
            create_checkout_session,
            import_license_file,
            get_license_state,
            remove_license,
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
