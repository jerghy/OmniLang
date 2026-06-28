// Tauri plugin commands — replaces Python FastAPI backend
// Routes compilation through the omnilang Rust crate for server-side operations.
// Frontend compilation goes through WASM (rustCompiler.ts); these commands provide
// complementary server-side support (project management, export, analysis).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Compile project ──

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub files: HashMap<String, String>,
    pub errors: Vec<String>,
}

#[tauri::command]
fn compile_project(project_path: String, target: String) -> CompileResult {
    // Convert target string to codegen::Target
    let codegen_target = match target.as_str() {
        "python" => omnilang::codegen::Target::Python,
        "js" | "javascript" => omnilang::codegen::Target::JavaScript,
        "ts" | "typescript" => omnilang::codegen::Target::TypeScript,
        "rust" | "rs" => omnilang::codegen::Target::Rust,
        "dart" => omnilang::codegen::Target::Dart,
        "react" => omnilang::codegen::Target::React,
        "vue" => omnilang::codegen::Target::Vue,
        "dioxus" => omnilang::codegen::Target::Dioxus,
        "flutter" => omnilang::codegen::Target::Flutter,
        "omniview" => omnilang::codegen::Target::OmniView,
        _ => {
            return CompileResult {
                success: false,
                files: HashMap::new(),
                errors: vec![format!("Unknown target: {}", target)],
            };
        }
    };

    // Discover .omni files in project directory
    let project_dir = std::path::Path::new(&project_path);
    let mut sources: HashMap<String, String> = HashMap::new();
    let mut discover_errors: Vec<String> = Vec::new();

    if project_dir.is_dir() {
        for entry in walkdir::WalkDir::new(project_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".omni"))
        {
            let path = entry.path().to_owned();
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let rel_path = path
                        .strip_prefix(project_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    sources.insert(rel_path, content);
                }
                Err(e) => {
                    discover_errors.push(format!("Cannot read {:?}: {}", path, e));
                }
            }
        }
    } else if project_dir.is_file() && project_dir.extension().map_or(false, |e| e == "omni") {
        match std::fs::read_to_string(project_dir) {
            Ok(content) => {
                let name = project_dir.file_name().unwrap().to_string_lossy().to_string();
                sources.insert(name, content);
            }
            Err(e) => {
                return CompileResult {
                    success: false,
                    files: HashMap::new(),
                    errors: vec![format!("Cannot read {:?}: {}", project_dir, e)],
                };
            }
        }
    } else {
        return CompileResult {
            success: false,
            files: HashMap::new(),
            errors: vec![format!("Project path not found: {}", project_path)],
        };
    }

    if sources.is_empty() {
        let mut all_errors = discover_errors;
        all_errors.push("No .omni source files found.".to_string());
        return CompileResult {
            success: false,
            files: HashMap::new(),
            errors: all_errors,
        };
    }

    // Build path mapping for cross-file imports
    let path_mapping = build_path_mapping(&sources);

    // Compile via omnilang crate
    let result_json = omnilang::compile_multi_to_target(sources, codegen_target, path_mapping);

    // Parse result JSON
    match serde_json::from_str::<CompileResult>(&result_json) {
        Ok(result) => result,
        Err(e) => CompileResult {
            success: false,
            files: HashMap::new(),
            errors: vec![format!("Failed to parse compile result: {}", e)],
        },
    }
}

// ── Analyse source ──

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyseResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

#[tauri::command]
fn analyse_source(source: String, file_path: String) -> AnalyseResult {
    let tokens = omnilang::lexer::tokenize(&source);
    let program = match omnilang::parser::parse(&tokens, &source) {
        Ok(p) => p,
        Err(e) => {
            return AnalyseResult {
                success: false,
                diagnostics: vec![Diagnostic {
                    severity: "error".to_string(),
                    code: "PARSE_ERROR".to_string(),
                    message: e.message,
                    file: file_path,
                    line: e.span.line,
                    col: e.span.col,
                }],
            };
        }
    };

    let semantics = omnilang::semantic::analyze(&program);
    let diagnostics: Vec<Diagnostic> = semantics
        .diagnostics
        .iter()
        .map(|d| Diagnostic {
            severity: match d.severity {
                omnilang::error::Severity::Error => "error".to_string(),
                omnilang::error::Severity::Warning => "warning".to_string(),
                omnilang::error::Severity::Info => "info".to_string(),
            },
            code: d.code.clone(),
            message: d.message.clone(),
            file: file_path.clone(),
            line: d.line,
            col: d.col,
        })
        .collect();

    let has_errors = diagnostics.iter().any(|d| d.severity == "error");
    AnalyseResult {
        success: !has_errors,
        diagnostics,
    }
}

// ── Project export ──

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub output_dir: String,
    pub file_count: u32,
    pub errors: Vec<String>,
}

#[tauri::command]
fn project_export(project_path: String, output_dir: String, target: String) -> ExportResult {
    // First compile the project
    let compile_result = compile_project(project_path.clone(), target.clone());

    if !compile_result.success {
        return ExportResult {
            success: false,
            output_dir: output_dir.clone(),
            file_count: 0,
            errors: compile_result.errors,
        };
    }

    // Write compiled files to output directory
    let out_path = std::path::Path::new(&output_dir);
    if let Err(e) = std::fs::create_dir_all(out_path) {
        return ExportResult {
            success: false,
            output_dir,
            file_count: 0,
            errors: vec![format!("Cannot create output directory: {}", e)],
        };
    }

    let mut written = 0u32;
    let mut write_errors: Vec<String> = Vec::new();

    for (filename, content) in &compile_result.files {
        let file_path = out_path.join(filename);
        if let Some(parent) = file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                write_errors.push(format!("Cannot create directory {:?}: {}", parent, e));
                continue;
            }
        }
        match std::fs::write(&file_path, content) {
            Ok(_) => written += 1,
            Err(e) => {
                write_errors.push(format!("Cannot write {:?}: {}", file_path, e));
            }
        }
    }

    ExportResult {
        success: write_errors.is_empty(),
        output_dir,
        file_count: written,
        errors: write_errors,
    }
}

// ── File system helpers ──

/// Build path mapping for cross-file import resolution.
fn build_path_mapping(sources: &HashMap<String, String>) -> HashMap<String, String> {
    let mut mapping: HashMap<String, String> = HashMap::new();

    for filepath in sources.keys() {
        // Direct path
        mapping.insert(filepath.clone(), filepath.clone());

        // Basename
        let basename = std::path::Path::new(filepath)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !basename.is_empty() && !mapping.contains_key(&basename) {
            mapping.insert(basename.clone(), filepath.clone());
        }

        // Stem (without extension)
        let stem = std::path::Path::new(&basename)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if !stem.is_empty() && !mapping.contains_key(&stem) {
            mapping.insert(stem, filepath.clone());
        }
    }

    // Compute relative path mappings for each pair
    for import_path in sources.keys() {
        let importer_path = std::path::Path::new(import_path);
        let importer_dir = importer_path.parent().unwrap_or(std::path::Path::new("."));

        for target_path in sources.keys() {
            if import_path == target_path {
                continue;
            }

            let target = std::path::Path::new(target_path);
            if let Ok(rel) = target.strip_prefix(importer_dir) {
                let rel_str = rel.to_string_lossy().to_string();
                // ./Component.omni
                mapping.insert(format!("./{}", rel_str), target_path.clone());
                // ../lib (without ./ prefix)
                mapping.insert(rel_str.clone(), target_path.clone());
                // ./Component (without .omni extension)
                if let Some(rel_stem) = rel.file_stem().and_then(|s| s.to_str()) {
                    mapping.insert(format!("./{}", rel_stem), target_path.clone());
                    mapping.insert(rel_stem.to_string(), target_path.clone());
                }
            }
        }
    }

    mapping
}

// ── Tauri app entry ──

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
            analyse_source,
            project_export,
        ])
        .plugin(victauri_plugin::init())
        .run(tauri::generate_context!())
        .expect("error while running Poiesis");
}
