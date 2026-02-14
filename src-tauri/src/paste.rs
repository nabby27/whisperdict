use anyhow::Result;
use arboard::Clipboard;
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key as EnigoKey, Keyboard, Settings,
};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub fn paste_text(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;

    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        let _ = Command::new("wtype")
            .args([
                "-M", "ctrl", "-M", "shift", "-k", "v", "-m", "shift", "-m", "ctrl",
            ])
            .status();
        return Ok(());
    }

    if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
        #[cfg(target_os = "macos")]
        let modifier = EnigoKey::Meta;
        #[cfg(not(target_os = "macos"))]
        let modifier = EnigoKey::Control;

        let _ = enigo.key(modifier, Press);
        #[cfg(target_os = "linux")]
        let _ = enigo.key(EnigoKey::Shift, Press);
        let _ = enigo.key(EnigoKey::Unicode('v'), Click);
        #[cfg(target_os = "linux")]
        let _ = enigo.key(EnigoKey::Shift, Release);
        let _ = enigo.key(modifier, Release);
        sleep(Duration::from_millis(20));
    }
    Ok(())
}
