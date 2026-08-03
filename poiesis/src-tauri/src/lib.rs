// Tauri plugin commands — replaces Python FastAPI backend
// Routes compilation through the omnilang Rust crate for server-side operations.
// Frontend compilation goes through WASM (rustCompiler.ts); these commands provide
// complementary server-side support (project management, export, analysis).

mod commands;
mod path;
mod state;

use commands::*;

/// Maximum source size for server-side analysis (10 MB).
pub(crate) const MAX_SOURCE_SIZE: usize = 10 * 1024 * 1024;

/// Convert an omnilang diagnostic to a JSON Value (useful for FFI or dynamic output).
#[allow(dead_code)]
#[cfg(feature = "server-compile")]
pub(crate) fn diagnostic_to_json(d: &omnilang::error::Diagnostic) -> serde_json::Value {
    serde_json::json!({
        "severity": match d.severity {
            omnilang::error::Severity::Error => "error",
            omnilang::error::Severity::Warning => "warning",
            omnilang::error::Severity::Info => "info",
        },
        "code": d.kind.code_str(),
        "message": d.message,
        "file": d.file,
        "line": d.line,
        "col": d.col,
        "suggestion": d.suggestion,
    })
}

// ── Tauri app entry ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["debug"])
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(state::ProjectState::default())
        .setup(|app| {
            // Pre-create the debug window hidden so open_debug_window can just show it.
            // WebviewWindowBuilder::build() blocks IPC responses on Windows,
            // so we create it here during setup (no IPC involved).
            // If it fails, open_debug_window will create it on demand.
            if let Err(e) = tauri::WebviewWindowBuilder::new(
                app,
                "debug",
                tauri::WebviewUrl::App("debug-window/index.html".into()),
            )
            .title("Poiesis Debug")
            .inner_size(1400.0, 900.0)
            .resizable(true)
            .visible(false)
            .build()
            {
                eprintln!("[Poiesis] Failed to pre-create debug window: {e}");
            }
            Ok(())
        })
        // Never destroy the debug window — hide it instead.
        // Recreating via WebviewWindowBuilder::build() blocks IPC responses
        // on Windows, which freezes the main window.
        .on_window_event(|window, event| {
            if window.label() == "debug" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window.hide().ok();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            #[cfg(feature = "server-compile")]
            compile_project,
            #[cfg(feature = "server-compile")]
            compile_source,
            #[cfg(feature = "server-compile")]
            analyse_source,
            #[cfg(feature = "server-compile")]
            project_export,
            get_project_dir,
            open_project,
            save_project,
            close_project,
            read_file,
            write_file,
            sync_store_state,
            get_store_snapshot,
            get_pid_index,
            #[cfg(feature = "server-compile")]
            verify_pid_roundtrip,
            open_debug_window,
            close_debug_window,
            dw_log,
        ]);

    #[cfg(debug_assertions)]
    let app = app.plugin(
        victauri_plugin::VictauriBuilder::new()
            .auth_disabled()
            .build()
            .expect("Failed to build Victauri plugin"),
    );

    let app = app
        .build(tauri::generate_context!())
        .expect("error while building Poiesis");
    app.run(|_app_handle, _event| {});
}
