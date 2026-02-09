use crate::app_state::AppState;
use anyhow::Result;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use std::env;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

enum Command {
    Update(String),
}

#[derive(Clone)]
pub struct WaylandHotkeys {
    tx: mpsc::Sender<Command>,
}

impl WaylandHotkeys {
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

    pub fn update(&self, shortcut: String) {
        let _ = self.tx.try_send(Command::Update(shortcut));
    }
}

async fn bind_shortcut(
    proxy: &GlobalShortcuts<'_>,
    session: &ashpd::desktop::Session<'_, GlobalShortcuts<'_>>,
    shortcut: &str,
) -> Result<()> {
    let shortcut = normalize_shortcut(shortcut);
    let shortcuts = [
        NewShortcut::new("toggle-recording", "Iniciar o detener Whisperdict")
            .preferred_trigger(Some(shortcut.as_str())),
    ];
    let request = proxy.bind_shortcuts(session, &shortcuts, None).await?;
    let _ = request.response()?;
    Ok(())
}

fn normalize_shortcut(input: &str) -> String {
    input.replace("Ctrl", "Control").replace("ALT", "Alt")
}
