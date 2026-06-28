use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<String>,
}

#[tauri::command]
fn compile_project(project_path: String, target: String) -> CompileResult {
    // TODO: 调用 omnilang crate 编译
    // 后续开发阶段实现具体逻辑
    CompileResult {
        success: true,
        output: format!("compiled {} -> {}", project_path, target),
        errors: vec![],
    }
}

#[tauri::command]
fn project_export(project_path: String, output_dir: String) -> String {
    format!("export {} -> {}", project_path, output_dir)
}

#[tauri::command]
fn file_watch(path: String) -> String {
    format!("watching {}", path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            compile_project,
            project_export,
            file_watch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Poiesis");
}
