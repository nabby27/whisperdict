use crate::app_state::AppState;
use anyhow::Result;
use rdev::{listen, Event, EventType, Key};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: Key,
}

impl Hotkey {
    pub fn parse(input: &str) -> Option<Self> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut key: Option<Key> = None;

        for part in input.split('+') {
            match part.trim().to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                "space" => key = Some(Key::Space),
                "a" => key = Some(Key::KeyA),
                "b" => key = Some(Key::KeyB),
                "c" => key = Some(Key::KeyC),
                "d" => key = Some(Key::KeyD),
                "e" => key = Some(Key::KeyE),
                "f" => key = Some(Key::KeyF),
                "g" => key = Some(Key::KeyG),
                "h" => key = Some(Key::KeyH),
                "i" => key = Some(Key::KeyI),
                "j" => key = Some(Key::KeyJ),
                "k" => key = Some(Key::KeyK),
                "l" => key = Some(Key::KeyL),
                "m" => key = Some(Key::KeyM),
                "n" => key = Some(Key::KeyN),
                "o" => key = Some(Key::KeyO),
                "p" => key = Some(Key::KeyP),
                "q" => key = Some(Key::KeyQ),
                "r" => key = Some(Key::KeyR),
                "s" => key = Some(Key::KeyS),
                "t" => key = Some(Key::KeyT),
                "u" => key = Some(Key::KeyU),
                "v" => key = Some(Key::KeyV),
                "w" => key = Some(Key::KeyW),
                "x" => key = Some(Key::KeyX),
                "y" => key = Some(Key::KeyY),
                "z" => key = Some(Key::KeyZ),
                _ => {}
            }
        }

        key.map(|key| Self {
            ctrl,
            alt,
            shift,
            key,
        })
    }
}

#[derive(Default)]
struct Modifiers {
    ctrl: bool,
    alt: bool,
    shift: bool,
}

pub fn start_listener(app: AppHandle, hotkey: Arc<Mutex<Hotkey>>) -> Result<()> {
    thread::spawn(move || {
        let modifiers = Arc::new(Mutex::new(Modifiers::default()));
        let mods_ref = modifiers.clone();
        let hotkey_ref = hotkey.clone();

        let callback = move |event: Event| {
            if let Ok(mut mods) = mods_ref.lock() {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        update_mods(key, true, &mut mods);
                        let current = hotkey_ref.lock().ok().map(|h| h.clone());
                        if let Some(hotkey) = current {
                            if hotkey.key == key
                                && hotkey.ctrl == mods.ctrl
                                && hotkey.alt == mods.alt
                                && hotkey.shift == mods.shift
                            {
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
                    EventType::KeyRelease(key) => {
                        update_mods(key, false, &mut mods);
                    }
                    _ => {}
                }
            }
        };

        let _ = listen(callback);
    });

    Ok(())
}

fn update_mods(key: Key, pressed: bool, mods: &mut Modifiers) {
    match key {
        Key::ControlLeft | Key::ControlRight => mods.ctrl = pressed,
        Key::ShiftLeft | Key::ShiftRight => mods.shift = pressed,
        Key::Alt | Key::AltGr => mods.alt = pressed,
        _ => {}
    }
}
