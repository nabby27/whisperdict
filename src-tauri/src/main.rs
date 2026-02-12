// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn configure_linux_runtime() {
    if std::env::var_os("GIO_USE_VFS").is_none() {
        std::env::set_var("GIO_USE_VFS", "local");
    }
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    configure_linux_runtime();

    if let Ok(true) = eco_lib::run_child() {
        return;
    }
    eco_lib::run()
}
