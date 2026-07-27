//! Debug window lifecycle and logging commands.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::Manager;

/// Open the debug window. Creates it if it doesn't exist yet.
#[tauri::command]
pub(crate) fn open_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    // Window was not pre-created — create it now
    tauri::WebviewWindowBuilder::new(
        &app,
        "debug",
        tauri::WebviewUrl::App("debug-window/index.html".into()),
    )
    .title("Poiesis Debug")
    .inner_size(1400.0, 900.0)
    .resizable(true)
    .visible(true)
    .build()
    .map_err(|e| format!("Failed to create debug window: {e}"))?;
    Ok(())
}

/// Hide the debug window (never destroy — recreate blocks IPC).
#[tauri::command]
pub(crate) fn close_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Append a line to the debug log file for frontend debugging.
#[tauri::command]
pub(crate) fn dw_log(message: String) -> Result<(), String> {
    let path = std::env::temp_dir().join("poiesis-debug.log");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open log file: {e}"))?;
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let secs = (ms / 1000) % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let millis = ms % 1000;
    writeln!(file, "[{h:02}:{m:02}:{s:02}.{millis:03}] {message}")
        .map_err(|e| format!("Failed to write log: {e}"))?;
    Ok(())
}
