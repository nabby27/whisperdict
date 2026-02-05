use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::Menu;
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrayMode {
    Idle,
    Recording,
    Processing,
    Error,
}

#[derive(Clone)]
pub struct TrayController {
    mode: Arc<Mutex<TrayMode>>,
    tray: Arc<Mutex<Option<TrayIcon>>>,
}

impl TrayController {
    pub fn new() -> Self {
        Self {
            mode: Arc::new(Mutex::new(TrayMode::Idle)),
            tray: Arc::new(Mutex::new(None)),
        }
    }

    pub fn init(&self, app: &AppHandle) {
        let menu = match Menu::new(app) {
            Ok(menu) => menu,
            Err(_) => return,
        };
        let icon = build_icon(TrayMode::Idle, true);
        let tray = TrayIconBuilder::new()
            .icon(icon)
            .menu(&menu)
            .on_tray_icon_event(|_tray, _event: TrayIconEvent| {})
            .build(app)
            .ok();
        if let Ok(mut guard) = self.tray.lock() {
            *guard = tray;
        }
    }

    pub fn set_mode(&self, mode: TrayMode) {
        if let Ok(mut guard) = self.mode.lock() {
            *guard = mode;
        }
        let icon = build_icon(mode, true);
        if let Ok(guard) = self.tray.lock() {
            if let Some(tray) = guard.as_ref() {
                let _ = tray.set_icon(Some(icon));
            }
        }
    }

    pub fn start_animation(&self) {
        let mode_ref = self.mode.clone();
        let tray_ref = self.tray.clone();
        tauri::async_runtime::spawn(async move {
            let mut pulse = false;
            loop {
                let mode = mode_ref.lock().map(|g| *g).unwrap_or(TrayMode::Idle);
                if mode == TrayMode::Recording {
                    pulse = !pulse;
                    let icon = build_icon(mode, pulse);
                    if let Ok(guard) = tray_ref.lock() {
                        if let Some(tray) = guard.as_ref() {
                            let _ = tray.set_icon(Some(icon));
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(520)).await;
            }
        });
    }
}

fn build_icon(mode: TrayMode, pulse: bool) -> Image<'static> {
    let size = 16u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let (r, g, b) = match mode {
        TrayMode::Idle => (160, 170, 180),
        TrayMode::Recording => {
            if pulse {
                (242, 108, 79)
            } else {
                (255, 184, 169)
            }
        }
        TrayMode::Processing => (240, 164, 75),
        TrayMode::Error => (225, 68, 68),
    };

    let radius = 6i32;
    let center = 7i32;
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = x - center;
            let dy = y - center;
            if dx * dx + dy * dy <= radius * radius {
                let idx = ((y as u32 * size + x as u32) * 4) as usize;
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = 255;
            }
        }
    }

    Image::new_owned(data, size, size)
}
