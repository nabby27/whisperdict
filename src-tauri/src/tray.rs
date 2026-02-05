use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const ICON_SIZE: u32 = 16;
const FRAME_MS: u64 = 140;

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
        let icon = render_icon(TrayMode::Idle, 0);
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
        let icon = render_icon(mode, 0);
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
            let mut last_mode = TrayMode::Idle;
            loop {
                let mode = mode_ref.lock().map(|g| *g).unwrap_or(TrayMode::Idle);
                if mode != last_mode {
                    frame = 0;
                    last_mode = mode;
                    let icon = render_icon(mode, 0);
                    if let Ok(guard) = tray_ref.lock() {
                        if let Some(tray) = guard.as_ref() {
                            let _ = tray.set_icon(Some(icon));
                        }
                    }
                }

                if mode == TrayMode::Recording || mode == TrayMode::Processing {
                    frame = frame.wrapping_add(1);
                    let icon = render_icon(mode, frame);
                    if let Ok(guard) = tray_ref.lock() {
                        if let Some(tray) = guard.as_ref() {
                            let _ = tray.set_icon(Some(icon));
                        }
                    }
                }

                tokio::time::sleep(Duration::from_millis(FRAME_MS)).await;
            }
        });
    }
}

fn render_icon(mode: TrayMode, frame: u8) -> Image<'static> {
    let mut data = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    clear(&mut data);

    match mode {
        TrayMode::Idle => draw_eco(&mut data, ICON_SIZE, (250, 250, 250, 255)),
        TrayMode::Error => draw_eco(&mut data, ICON_SIZE, (243, 18, 96, 255)),
        TrayMode::Recording => draw_recording(&mut data, ICON_SIZE, frame),
        TrayMode::Processing => draw_processing(&mut data, ICON_SIZE, frame),
    }

    Image::new_owned(data, ICON_SIZE, ICON_SIZE)
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

fn draw_letter(
    data: &mut [u8],
    size: u32,
    x0: i32,
    y0: i32,
    rows: &[u8; 7],
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    for (row_idx, row) in rows.iter().enumerate() {
        for col in 0..4 {
            if row & (1 << (3 - col)) != 0 {
                set_pixel(data, size, x0 + col as i32, y0 + row_idx as i32, r, g, b, a);
            }
        }
    }
}

fn draw_recording(data: &mut [u8], size: u32, frame: u8) {
    let center = (size as i32 - 1) / 2;
    let bars = [1, 3, 5, 7, 9, 11];
    let frames: [[i32; 6]; 12] = [
        [4, 7, 9, 8, 6, 4],
        [5, 8, 10, 7, 5, 6],
        [6, 6, 9, 11, 6, 5],
        [4, 7, 8, 10, 7, 6],
        [5, 9, 11, 9, 5, 4],
        [6, 8, 10, 8, 6, 5],
        [4, 6, 9, 11, 7, 6],
        [5, 7, 8, 9, 6, 5],
        [6, 9, 10, 8, 5, 4],
        [4, 8, 11, 10, 6, 5],
        [5, 7, 9, 8, 7, 6],
        [6, 8, 10, 9, 5, 4],
    ];
    let heights = frames[(frame as usize) % frames.len()];

    for (i, x) in bars.iter().enumerate() {
        let h = heights[i];
        let top = center - h / 2;
        let bottom = center + h / 2;
        for y in top..=bottom {
            set_pixel(data, size, *x, y, 255, 255, 255, 255);
        }
    }
}

fn draw_processing(data: &mut [u8], size: u32, frame: u8) {
    let center = (size as f32 - 1.0) / 2.0;
    let radius = (size as f32 / 2.0) - 2.5;
    let thickness = 1.4f32;
    let start = (frame as f32 * 18.0) % 360.0;
    let arc = 110.0 + ((frame as f32 * 0.12).sin() + 1.0) * 35.0;
    let base_color = (159, 179, 240, 255);
    let arc_color = (78, 105, 212, 255);

    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let edge = (dist - radius).abs();
            if edge > thickness {
                continue;
            }
            let mut angle = dy.atan2(dx) * 180.0 / std::f32::consts::PI;
            if angle < 0.0 {
                angle += 360.0;
            }
            let in_arc = angle_in_arc(angle, start, arc);
            let feather = 1.0 - (edge / thickness).min(1.0);
            let (r, g, b, a) = if in_arc { arc_color } else { base_color };
            let blended = (a as f32 * feather).round() as u8;
            set_pixel(data, size, x, y, r, g, b, blended);
        }
    }
}

fn angle_in_arc(angle: f32, start: f32, arc: f32) -> bool {
    let end = (start + arc) % 360.0;
    if start <= end {
        angle >= start && angle <= end
    } else {
        angle >= start || angle <= end
    }
}

#[cfg(test)]
mod tests {
    use super::{render_icon, TrayMode};

    fn opaque_pixels(data: &[u8]) -> usize {
        data.chunks_exact(4).filter(|px| px[3] > 0).count()
    }

    #[test]
    fn idle_icon_renders_text() {
        let image = render_icon(TrayMode::Idle, 0);
        assert!(opaque_pixels(image.rgba()) > 20);
    }

    #[test]
    fn recording_frames_change() {
        let a = render_icon(TrayMode::Recording, 1).rgba().to_vec();
        let b = render_icon(TrayMode::Recording, 8).rgba().to_vec();
        assert_ne!(a, b);
        assert!(opaque_pixels(&a) > 20);
    }

    #[test]
    fn processing_frames_change() {
        let a = render_icon(TrayMode::Processing, 1).rgba().to_vec();
        let b = render_icon(TrayMode::Processing, 10).rgba().to_vec();
        assert_ne!(a, b);
        assert!(opaque_pixels(&a) > 20);
    }
}
