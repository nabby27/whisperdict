use crate::app_state::AppState;
use anyhow::Result;
#[cfg(target_os = "linux")]
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
#[cfg(target_os = "linux")]
use futures_util::StreamExt;
#[cfg(target_os = "linux")]
use std::env;
use tauri::{AppHandle, Manager};
#[cfg(target_os = "linux")]
use tokio::sync::mpsc;

#[cfg(target_os = "linux")]
enum Command {
    Update(String),
}

#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct WaylandHotkeys {
    tx: mpsc::Sender<Command>,
}

#[cfg(not(target_os = "linux"))]
#[derive(Clone)]
pub struct WaylandHotkeys;

impl WaylandHotkeys {
    #[cfg(target_os = "linux")]
    pub fn start(app: AppHandle, shortcut: String) -> Option<Self> {
        if env::var("WAYLAND_DISPLAY").is_err() {
            return None;
        }

        let (tx, mut rx) = mpsc::channel::<Command>(8);
        tauri::async_runtime::spawn(async move {
            let proxy = match GlobalShortcuts::new().await {
                Ok(proxy) => proxy,
                Err(_) => return,
            };

            let session = match proxy.create_session().await {
                Ok(session) => session,
                Err(_) => return,
            };

            let mut current = shortcut;
            let _ = bind_shortcut(&proxy, &session, &current).await;

            let mut activated = match proxy.receive_activated().await {
                Ok(stream) => stream,
                Err(_) => return,
            };

            loop {
                tokio::select! {
                    Some(cmd) = rx.recv() => {
                        let Command::Update(next) = cmd;
                        current = next;
                        let _ = bind_shortcut(&proxy, &session, &current).await;
                    }
                    event = activated.next() => {
                        if let Some(event) = event {
                            if event.shortcut_id() == "toggle-recording" {
                                let app_handle = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    let state = app_handle.state::<AppState>();
                                    let recording = state.status().recording;
                                    if recording {
                                        let _ = state.stop_recording(&app_handle).await;
                                    } else {
                                        let _ = state.start_recording(&app_handle);
                                    }
                                });
                            }
                        }
                    }
                }
            }
        });

        Some(Self { tx })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn start(_app: AppHandle, _shortcut: String) -> Option<Self> {
        None
    }

    #[cfg(target_os = "linux")]
    pub fn update(&self, shortcut: String) {
        let _ = self.tx.try_send(Command::Update(shortcut));
    }

    #[cfg(not(target_os = "linux"))]
    pub fn update(&self, _shortcut: String) {}
}

#[cfg(target_os = "linux")]
async fn bind_shortcut(
    proxy: &GlobalShortcuts<'_>,
    session: &ashpd::desktop::Session<'_, GlobalShortcuts<'_>>,
    shortcut: &str,
) -> Result<()> {
    let shortcut = normalize_shortcut(shortcut);
    let shortcuts = [
        NewShortcut::new("toggle-recording", "Start or stop Whisperdict")
            .preferred_trigger(Some(shortcut.as_str())),
    ];
    let request = proxy.bind_shortcuts(session, &shortcuts, None).await?;
    let _ = request.response()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn normalize_shortcut(input: &str) -> String {
    input.replace("Ctrl", "Control").replace("ALT", "Alt")
}
