//! Poiesis E2E tests — simulates real user workflows through the Victauri bridge.
//!
//! These tests exercise the full stack: Tauri backend → IPC → frontend UI.
//! They catch regressions that compile-time checks (cargo build, tsc, clippy) miss:
//!   - Tauri config deserialization (v1→v2 field name migration)
//!   - Plugin scope / capability path restrictions
//!   - Vite import resolution (CSS, modules)
//!   - UI rendering after project load
//!
//! Run with: VICTAURI_E2E=1 cargo test --test e2e -- --nocapture
//!
//! Requires a running Poiesis dev server (start-poiesis.sh).

use serde_json::json;
use victauri_test::VictauriClient;

fn skip_unless_e2e() -> bool {
    if !victauri_test::is_e2e() {
        eprintln!("Skipping: set VICTAURI_E2E=1 with your Tauri dev server running");
        return true;
    }
    false
}

const TEST_PROJECT: &str = "D:/space/omni/demo2";

// ── Health & Startup ────────────────────────────────────────────────────────

/// Catches: Tauri config deserialization panics, Vite import errors,
/// CSP violations, plugin init failures.
#[tokio::test]
async fn app_starts_and_is_healthy() {
    if skip_unless_e2e() {
        return;
    }
    let mut client = VictauriClient::discover()
        .await
        .expect("Failed to connect — is Poiesis running?");

    // Plugin info confirms the app initialized without panicking
    let info = client.get_plugin_info().await.unwrap();
    let version = info
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    eprintln!("Connected to Victauri v{}", version);
    assert!(info.get("version").is_some(), "plugin_info should have version");

    // Full health report — IPC integrity + no console errors
    // console errors would include Vite "Failed to resolve import" overlays
    let report = client
        .verify()
        .ipc_healthy()
        .no_console_errors()
        .run()
        .await
        .unwrap();

    for result in &report.results {
        eprintln!(
            "  [{}] {}",
            if result.passed { "PASS" } else { "FAIL" },
            result.description
        );
    }
    report.assert_all_passed();
}

// ── Project Open via Tauri Commands ─────────────────────────────────────────

/// Catches: fs:scope too narrow (forbidden path errors on non-HOME drives).
/// Opens a project at D:/space/omni/demo2 and reads its .omni files
/// through Tauri commands — the backend path that was blocked before
/// we expanded the capability scope.
#[tokio::test]
async fn open_project_and_read_files_via_commands() {
    if skip_unless_e2e() {
        return;
    }
    let mut client = VictauriClient::discover().await.unwrap();

    // 1. Open the project
    let result = client
        .invoke_command("open_project", Some(json!({"path": TEST_PROJECT})))
        .await
        .expect("open_project should succeed");
    assert!(
        result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
        "open_project should return success: {:?}",
        result
    );

    // 2. Verify project dir is set
    let dir = client
        .invoke_command("get_project_dir", None)
        .await
        .expect("get_project_dir should succeed");
    let dir_str = dir.as_str().unwrap_or("");
    assert!(
        dir_str.contains("demo2"),
        "project dir should be demo2, got: {}",
        dir_str
    );
    eprintln!("Project dir: {}", dir_str);

    // 3. Read the main .omni file — this exercises the fs plugin scope
    let content = client
        .invoke_command("read_file", Some(json!({"path": "src/main.omni"})))
        .await
        .expect("read_file src/main.omni should succeed");
    let text = content.as_str().unwrap_or("");
    assert!(
        text.contains("component main"),
        "main.omni should contain 'component main', got {} bytes",
        text.len()
    );
    eprintln!("main.omni: {} bytes", text.len());

    // 4. Read a component file from a subdirectory
    let comp = client
        .invoke_command(
            "read_file",
            Some(json!({"path": "src/components/MyCard.omni"})),
        )
        .await
        .expect("read_file src/components/MyCard.omni should succeed");
    let comp_text = comp.as_str().unwrap_or("");
    assert!(
        comp_text.contains("component MyCard"),
        "MyCard.omni should contain 'component MyCard', got {} bytes",
        comp_text.len()
    );
    eprintln!("MyCard.omni: {} bytes", comp_text.len());

    // 5. Close the project
    let closed = client
        .invoke_command("close_project", None)
        .await
        .expect("close_project should succeed");
    assert!(
        closed
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        "close_project should return success"
    );
}

// ── UI: Project Opens and Shows Tabs ────────────────────────────────────────

/// Catches: fs scope issues manifesting as UI failures (no tabs, empty canvas).
/// Simulates the real user flow: welcome page → click project → tabs appear.
/// This is the most realistic test — it exercises the exact path that failed
/// when demo2 couldn't be read from D:/.
#[tokio::test]
async fn ui_project_opens_and_shows_tabs() {
    if skip_unless_e2e() {
        return;
    }
    let mut client = VictauriClient::discover().await.unwrap();

    // Ensure we start from the welcome page (previous tests may have
    // left the editor open). If the editor is visible, close the
    // project and reload.
    let is_editor = client
        .eval_js("document.body.innerText.includes('组件面板')")
        .await
        .unwrap();
    if is_editor.as_bool().unwrap_or(false) {
        eprintln!("Editor is open from a previous test — returning to welcome");
        client
            .invoke_command("close_project", None)
            .await
            .unwrap();
        client.eval_js("window.location.reload()").await.unwrap();
        client
            .expect_text_with_timeout("打开项目", 5000)
            .await
            .unwrap();
    }

    // 1. We should be on the welcome page (no project open)
    let on_welcome = client
        .eval_js(
            "document.body.innerText.includes('Poiesis') &&
             document.body.innerText.includes('打开项目')",
        )
        .await
        .unwrap();
    assert!(
        on_welcome.as_bool().unwrap_or(false),
        "Should start on welcome page"
    );
    eprintln!("On welcome page ✓");

    // 2. Click the demo2 recent project button
    client
        .click_by_text("demo2")
        .await
        .expect("Should find and click demo2 button");

    // 3. Wait for IPC to settle (file scanning completes)
    client
        .wait_for("ipc_idle", None, Some(10000), None)
        .await
        .unwrap();

    // 4. Verify tabs appeared — main.omni and MyCard.omni should be visible
    client
        .expect_text_with_timeout("main.omni", 5000)
        .await
        .expect("Tab 'main.omni' should appear after opening project");
    client
        .expect_text_with_timeout("MyCard.omni", 3000)
        .await
        .expect("Tab 'MyCard.omni' should appear after opening project");
    eprintln!("Tabs appeared ✓");

    // 5. Verify the canvas is NOT empty (no "空画布" placeholder)
    client
        .expect_no_text("空画布")
        .await
        .expect("Canvas should not be empty");
    eprintln!("Canvas populated ✓");

    // 6. Verify the StatusBar shows the project info
    client
        .expect_text_with_timeout("demo2", 3000)
        .await
        .expect("StatusBar should show project name");

    // 7. Verify no console errors accumulated
    let report = client.verify().no_console_errors().run().await.unwrap();
    report.assert_all_passed();
    eprintln!("No console errors ✓");
}

// ── File Read/Write Roundtrip ───────────────────────────────────────────────

/// Catches: fs write permission issues, path sanitization regressions.
/// Writes a file then reads it back, asserting content matches.
#[tokio::test]
async fn file_read_write_roundtrip() {
    if skip_unless_e2e() {
        return;
    }
    let mut client = VictauriClient::discover().await.unwrap();

    // 1. Open project
    client
        .invoke_command("open_project", Some(json!({"path": TEST_PROJECT})))
        .await
        .unwrap();

    // 2. Write a test file
    let test_content = "// E2E roundtrip test\ncomponent TestRoundtrip {\n    view { <div/> }\n}\n";
    let write_result = client
        .invoke_command(
            "write_file",
            Some(json!({
                "path": "src/components/__e2e_test.omni",
                "content": test_content
            })),
        )
        .await
        .expect("write_file should succeed");
    assert!(
        write_result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        "write_file should return success"
    );

    // 3. Read it back
    let read_result = client
        .invoke_command(
            "read_file",
            Some(json!({"path": "src/components/__e2e_test.omni"})),
        )
        .await
        .expect("read_file should succeed");
    let read_text = read_result.as_str().unwrap_or("");
    assert_eq!(
        read_text, test_content,
        "Read content should match written content"
    );
    eprintln!("Roundtrip OK — {} bytes match", test_content.len());

    // 4. Clean up — close project so file cleanup can happen
    client
        .invoke_command("close_project", None)
        .await
        .unwrap();
}

// ── Close and Reopen Project ────────────────────────────────────────────────

/// Catches: state leaks, project_dir not cleared, stale tab data.
/// Closes a project (back to welcome), then reopens it (tabs restore).
#[tokio::test]
async fn close_and_reopen_project() {
    if skip_unless_e2e() {
        return;
    }
    let mut client = VictauriClient::discover().await.unwrap();

    // ── Round 1: Open ──
    client
        .click_by_text("demo2")
        .await
        .expect("Should find and click demo2");
    client
        .wait_for("ipc_idle", None, Some(10000), None)
        .await
        .unwrap();
    client
        .expect_text_with_timeout("main.omni", 5000)
        .await
        .expect("Tab should appear (round 1)");
    eprintln!("Round 1: project opened ✓");

    // ── Navigate back to welcome ──
    // The app doesn't have a "close project" button in the UI,
    // so we call the Tauri command + reload the frontend state.
    client
        .invoke_command("close_project", None)
        .await
        .unwrap();

    // Reload the page to reset frontend state
    client
        .eval_js("window.location.reload()")
        .await
        .unwrap();

    // Wait for the welcome page to render
    client
        .expect_text_with_timeout("打开项目", 5000)
        .await
        .expect("Welcome page should reappear after close");
    eprintln!("Welcome page restored ✓");

    // ── Round 2: Reopen the same project ──
    client
        .click_by_text("demo2")
        .await
        .expect("Should find and click demo2 (round 2)");
    client
        .wait_for("ipc_idle", None, Some(10000), None)
        .await
        .unwrap();
    client
        .expect_text_with_timeout("main.omni", 5000)
        .await
        .expect("Tab should reappear after reopen");
    client
        .expect_text_with_timeout("MyCard.omni", 3000)
        .await
        .expect("Component tab should reappear after reopen");
    eprintln!("Round 2: project reopened ✓");

    // Final health check
    let report = client
        .verify()
        .ipc_healthy()
        .no_console_errors()
        .run()
        .await
        .unwrap();
    report.assert_all_passed();
}
