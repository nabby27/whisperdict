use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

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
        let show_item = match MenuItem::with_id(app, "show", "Show", true, None::<&str>) {
            Ok(item) => item,
            Err(_) => return,
        };
        let quit_item = match MenuItem::with_id(app, "quit", "Quit", true, None::<&str>) {
            Ok(item) => item,
            Err(_) => return,
        };
        let menu = match MenuBuilder::new(app).items(&[&show_item, &quit_item]).build() {
            Ok(menu) => menu,
            Err(_) => return,
        };
        let icon = build_icon(TrayMode::Idle, 0);
        let tray = TrayIconBuilder::new()
            .icon(icon)
            .menu(&menu)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
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
        let icon = build_icon(mode, 0);
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
            let mut frame: u8 = 0;
            loop {
                let mode = mode_ref.lock().map(|g| *g).unwrap_or(TrayMode::Idle);
                if mode == TrayMode::Recording || mode == TrayMode::Processing {
                    frame = frame.wrapping_add(1);
                    let icon = build_icon(mode, frame);
                    if let Ok(guard) = tray_ref.lock() {
                        if let Some(tray) = guard.as_ref() {
                            let _ = tray.set_icon(Some(icon));
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(140)).await;
            }
        });
    }
}

fn build_icon(mode: TrayMode, frame: u8) -> Image<'static> {
    let size = 16u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    clear(&mut data);

    match mode {
        TrayMode::Idle => draw_eco(&mut data, size, (250, 250, 250, 255)),
        TrayMode::Error => draw_eco(&mut data, size, (243, 18, 96, 255)),
        TrayMode::Recording => draw_bars(&mut data, size, frame, (0, 112, 243, 255)),
        TrayMode::Processing => draw_spinner(&mut data, size, frame, (0, 112, 243, 255)),
    }

    Image::new_owned(data, size, size)
}

fn clear(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        pixel[0] = 0;
        pixel[1] = 0;
        pixel[2] = 0;
        pixel[3] = 0;
    }
}

fn set_pixel(data: &mut [u8], size: u32, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
        return;
    }
    let idx = ((y as u32 * size + x as u32) * 4) as usize;
    data[idx] = r;
    data[idx + 1] = g;
    data[idx + 2] = b;
    data[idx + 3] = a;
}

fn draw_eco(data: &mut [u8], size: u32, color: (u8, u8, u8, u8)) {
    let (r, g, b, a) = color;
    let e = [0b1111, 0b1000, 0b1000, 0b1110, 0b1000, 0b1000, 0b1111];
    let c = [0b0111, 0b1000, 0b1000, 0b1000, 0b1000, 0b1000, 0b0111];
    let o = [0b0110, 0b1001, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110];
    draw_letter(data, size, 1, 4, &e, r, g, b, a);
    draw_letter(data, size, 6, 4, &c, r, g, b, a);
    draw_letter(data, size, 11, 4, &o, r, g, b, a);
}

fn draw_letter(data: &mut [u8], size: u32, x0: i32, y0: i32, rows: &[u8; 7], r: u8, g: u8, b: u8, a: u8) {
    for (row_idx, row) in rows.iter().enumerate() {
        for col in 0..4 {
            if row & (1 << (3 - col)) != 0 {
                set_pixel(data, size, x0 + col as i32, y0 + row_idx as i32, r, g, b, a);
            }
        }
    }
}

fn draw_bars(data: &mut [u8], size: u32, frame: u8, color: (u8, u8, u8, u8)) {
    let (r, g, b, a) = color;
    let bars = [2, 4, 6, 8, 10, 12, 14];
    let heights = [3, 5, 7, 9, 7, 5, 3, 4, 6, 8, 6, 4];
    let center = 8i32;
    for (i, x) in bars.iter().enumerate() {
        let idx = (frame as usize + i * 2) % heights.len();
        let h = heights[idx] as i32;
        for y in (center - h / 2)..=(center + h / 2) {
            set_pixel(data, size, *x as i32, y, r, g, b, a);
        }
    }
}

fn draw_spinner(data: &mut [u8], size: u32, frame: u8, color: (u8, u8, u8, u8)) {
    let (r, g, b, a) = color;
    let center = 8i32;
    let points = [
        (8, 2), (10, 3), (12, 5), (13, 8), (12, 11), (10, 13), (8, 14), (6, 13),
        (4, 11), (3, 8), (4, 5), (6, 3),
    ];
    let start = (frame as usize) % points.len();
    for i in 0..points.len() {
        let idx = (start + i) % points.len();
        let (x, y) = points[idx];
        if i < 5 {
            set_pixel(data, size, x, y, r, g, b, a);
        }
    }
    set_pixel(data, size, center, center, r, g, b, a);
}
