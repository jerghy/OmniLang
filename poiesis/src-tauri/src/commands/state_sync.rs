//! State-sync and PID-verification commands.

use crate::path::sanitize_path;
use crate::state::ProjectState;

// ── PID verification ──

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct PidRoundtripResult {
    consistent: bool,
    pids: Vec<String>,
}

#[tauri::command]
pub(crate) fn sync_store_state(
    store_snapshot: String,
    pid_index: String,
    state: tauri::State<ProjectState>,
) -> Result<(), String> {
    *state.store_snapshot.lock().map_err(|e| e.to_string())? = store_snapshot;
    *state.pid_index.lock().map_err(|e| e.to_string())? = pid_index;
    Ok(())
}

#[tauri::command]
pub(crate) fn get_store_snapshot(state: tauri::State<ProjectState>) -> Result<String, String> {
    let snap = state
        .store_snapshot
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    if snap.is_empty() {
        Ok("{}".to_string())
    } else {
        Ok(snap)
    }
}

#[tauri::command]
pub(crate) fn get_pid_index(state: tauri::State<ProjectState>) -> Result<String, String> {
    Ok(state.pid_index.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub(crate) fn verify_pid_roundtrip(
    file: String,
    state: tauri::State<ProjectState>,
) -> Result<serde_json::Value, String> {
    let dir = state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())?;
    let full = sanitize_path(std::path::Path::new(&dir), &file)?;
    let source = std::fs::read_to_string(&full)
        .map_err(|e| format!("Failed to read {}: {}", full.display(), e))?;

    // Parse using omnilang
    let tokens = omnilang::lexer::tokenize(&source);
    let (program, parse_errors) = omnilang::parser::parse(tokens, &source);
    if !parse_errors.is_empty() {
        let msgs: Vec<String> = parse_errors.iter().map(|e| e.message.clone()).collect();
        return Ok(serde_json::json!({
            "consistent": false,
            "pids": [],
            "error": msgs.join("; ")
        }));
    }

    // Extract @pid annotations from comments in source
    let mut pids_from_comments: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("// @pid:") {
            let pid = trimmed[pos + 8..].trim().to_string();
            if !pid.is_empty() && !pids_from_comments.contains(&pid) {
                pids_from_comments.push(pid);
            }
        }
    }

    // Count components in the parsed program
    let mut component_count = 0usize;
    for module in program.modules.values() {
        component_count += module.components.len();
    }

    // Consistent if: file parses successfully AND has @pid annotations
    let consistent = !pids_from_comments.is_empty();

    Ok(serde_json::json!({
        "consistent": consistent,
        "pids": pids_from_comments,
        "component_count": component_count,
    }))
}
