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
        if Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .status()
            .is_ok()
        {
            return Ok(());
        }
        let _ = Command::new("wtype")
            .args([
                "-M", "ctrl", "-M", "shift", "-k", "v", "-m", "shift", "-m", "ctrl",
            ])
            .status();
        let _ = Command::new("wtype")
            .args(["-M", "shift", "-k", "Insert", "-m", "shift"])
            .status();
        return Ok(());
    }

    if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
        let _ = enigo.key(EnigoKey::Control, Press);
        let _ = enigo.key(EnigoKey::Unicode('v'), Click);
        let _ = enigo.key(EnigoKey::Control, Release);
        sleep(Duration::from_millis(20));
        let _ = enigo.key(EnigoKey::Control, Press);
        let _ = enigo.key(EnigoKey::Shift, Press);
        let _ = enigo.key(EnigoKey::Unicode('v'), Click);
        let _ = enigo.key(EnigoKey::Shift, Release);
        let _ = enigo.key(EnigoKey::Control, Release);
        sleep(Duration::from_millis(20));
        let _ = enigo.key(EnigoKey::Shift, Press);
        let _ = enigo.key(EnigoKey::Insert, Click);
        let _ = enigo.key(EnigoKey::Shift, Release);
    }
    Ok(())
}
