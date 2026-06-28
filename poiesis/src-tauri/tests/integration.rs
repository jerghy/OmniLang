//! Integration tests — auto-generated from discovered #[tauri::command] functions.
//!
//! Run with: VICTAURI_E2E=1 cargo test --test integration

use serde_json::json;
use victauri_test::{e2e_test, VictauriClient};

fn skip_unless_e2e() -> bool {
    if !victauri_test::is_e2e() {
        eprintln!("Skipping: set VICTAURI_E2E=1 with your Tauri dev server running");
        return true;
    }
    false
}

// ── Health check ─────────────────────────────────────────────────────────

#[tokio::test]
async fn full_stack_health_check() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await
        .expect("Failed to connect — is your Tauri dev server running?");

    let report = client.verify()
        .ipc_healthy()
        .no_console_errors()
        .run()
        .await
        .unwrap();

    report.assert_all_passed();
}

// ── Command tests ────────────────────────────────────────────────────────
// Generated from discovered #[tauri::command] functions.
// Adapt each test to match your command's expected arguments and behavior.

#[tokio::test]
async fn command_compile_project() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("compile_project", None).await;
    assert!(
        result.is_ok(),
        "compile_project should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_file_watch() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("file_watch", None).await;
    assert!(
        result.is_ok(),
        "file_watch should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_project_export() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("project_export", None).await;
    assert!(
        result.is_ok(),
        "project_export should respond without error: {:?}",
        result.err()
    );
}

