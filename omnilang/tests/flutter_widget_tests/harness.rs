//! Flutter widget-test harness — first runtime verification of generated
//! code (Phase 3.2 / flutter 长期项 #2).
//!
//! Regenerates the learn-omniview Flutter output project
//! (`test/projects/learn-omniview/build/flutter-test-app/`) from the
//! CURRENT codegen, materializes the smoke tests (`smoke/*.dart` →
//! `<app>/test/`), then runs the runtime gates when a Flutter SDK is
//! available:
//!
//! 1. `flutter pub get` + `flutter analyze` — the dart-analyze gate
//!    (acceptance: no flutter-gate regression). Always runs when the SDK
//!    is found; SKIPs (with a note) otherwise.
//! 2. `flutter test` — the widget-test smoke suite (app mount / @if /
//!    @for / TodoStore.add() / theme / page switch / motion triggers).
//!    Opt-in via `OMNILANG_FLUTTER_TEST=1`: `flutter test` adds ~30-60s
//!    wall clock, so it stays out of default `cargo test` runs. CI wiring
//!    landed in Task 7.6: the codegen-validate job runs this step with
//!    `OMNILANG_FLUTTER_TEST=1` (option A — see README.md in this
//!    directory), so the smoke suite is exercised on every push.
//!
//! Run:
//!   cargo test -p omnilang --features codegen-flutter --test flutter_widget_tests
//!   OMNILANG_FLUTTER_TEST=1 cargo test -p omnilang --features codegen-flutter --test flutter_widget_tests

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use omnilang::codegen::Target;
use omnilang::discovery::{discover_omni_sources, DiscoveryMode};
use omnilang::path_mapping::derive_path_mapping;

/// learn-omniview showcase source directory (relative to repo root).
const LEARN_OMNIVIEW_SRC: &str = "test/projects/learn-omniview/src";
/// Materialized Flutter output project (gitignored build artifact; its
/// `test/` directory is committed as a snapshot for review/CI).
const FLUTTER_TEST_APP: &str = "test/projects/learn-omniview/build/flutter-test-app";
/// Smoke test source of truth (committed) — copied into `<app>/test/`.
const SMOKE_DIR: &str = "tests/flutter_widget_tests/smoke";

/// pubspec pinned to the dependency set the generated code needs
/// (mirrors the generated_code_validation gate's pubspec — http / intl /
/// shared_preferences are imported by generated stdlib emits).
const FLUTTER_TEST_APP_PUBSPEC: &str = r#"name: flutter_test_app
environment:
  sdk: ^3.5.0
dependencies:
  flutter:
    sdk: flutter
  flutter_riverpod: ^3.0.0
  # Task 7 (批次 2): @route/@layout/@query + navigate/back emit GoRouter
  # imports — the generated code depends on go_router (pub.dev 官方最新).
  go_router: ^17.5.0
  http: ^1.2.0
  shared_preferences: ^2.2.0
  intl: ^0.19.0
dev_dependencies:
  flutter_test:
    sdk: flutter
"#;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("should have parent directory")
        .to_path_buf()
}

fn run(cmd: &mut Command) -> Output {
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to run {cmd:?}: {e}"))
}

fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}")
}

/// Locate the flutter executable. Mirrors `dart_command` in
/// generated_code_validation.rs — on Windows flutter ships a `.bat` shim
/// that needs `cmd /C` resolution (Rust's `Command` does not apply
/// PATHEXT on its own).
fn flutter_command() -> Option<Command> {
    if Command::new("flutter")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return Some(Command::new("flutter"));
    }
    #[cfg(windows)]
    if Command::new("cmd")
        .args(["/C", "flutter", "--version"])
        .output()
        .is_ok_and(|o| o.status.success())
    {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("flutter");
        return Some(cmd);
    }
    None
}

/// Run `flutter <args...>` in `dir` with a fresh command (Command keeps
/// previously added args, so each invocation must start from scratch).
fn flutter_run(dir: &Path, args: &[&str]) -> Output {
    let mut flutter = flutter_command().expect("flutter_command() checked by caller");
    flutter.args(args).current_dir(dir);
    run(&mut flutter)
}

/// Regenerate the widget-test app from current codegen, then run the
/// runtime gates (analyze always when the SDK is present; `flutter test`
/// when `OMNILANG_FLUTTER_TEST=1`).
#[test]
fn flutter_widget_tests() {
    // ── 1. Compile learn-omniview → Flutter with the CURRENT codegen ──
    let root = repo_root();
    let src_dir = root.join(LEARN_OMNIVIEW_SRC);
    let sources = discover_omni_sources(&src_dir, DiscoveryMode::Directory)
        .unwrap_or_else(|e| panic!("discover {src_dir:?}: {e}"));
    assert!(!sources.is_empty(), "learn-omniview must yield sources");

    let path_mapping = derive_path_mapping(&sources);
    let result = omnilang::compile_multi_internal(
        sources,
        Target::Flutter,
        path_mapping,
        false,
        false,
        None,
    );
    // The showcase project demonstrates DOM-only pointer-capture ref ops
    // (design D3). On Flutter those are a DELIBERATE hard error
    // ("Flutter 不支持指针捕获，用 GestureDetector" — spec §10.3), so the
    // project can never compile cleanly to Flutter while ElementRefDemo
    // keeps its pointer-capture section. Filter that expected diagnostic
    // out — mirroring generated_code_validation.rs; any other error is a
    // regression.
    let unexpected: Vec<String> = result
        .errors
        .iter()
        .filter(|d| !d.message.contains("Flutter 不支持指针捕获"))
        .map(|d| format!("{}:{} {}", d.line, d.col, d.message))
        .collect();
    assert!(
        unexpected.is_empty(),
        "learn-omniview compilation to Flutter failed: {:?}",
        unexpected
    );
    assert!(!result.files.is_empty(), "codegen produced no files");

    // ── 2. Materialize the widget-test app ──
    let app_dir = root.join(FLUTTER_TEST_APP);
    let lib_dir = app_dir.join("lib");
    let test_dir = app_dir.join("test");
    if lib_dir.exists() {
        std::fs::remove_dir_all(&lib_dir).expect("clear stale lib/");
    }
    std::fs::create_dir_all(&lib_dir).expect("create lib/");
    std::fs::create_dir_all(&test_dir).expect("create test/");
    for (path, content) in &result.files {
        std::fs::write(lib_dir.join(path), content).unwrap_or_else(|e| panic!("write {path}: {e}"));
    }
    std::fs::write(app_dir.join("pubspec.yaml"), FLUTTER_TEST_APP_PUBSPEC)
        .expect("write pubspec.yaml");

    // smoke/ lives inside the omnilang crate (CARGO_MANIFEST_DIR), not the
    // repo root — the generated app itself lives at repo-root paths above.
    let smoke_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SMOKE_DIR);
    let mut smoke_files: Vec<PathBuf> = std::fs::read_dir(&smoke_dir)
        .expect("smoke dir")
        .map(|e| e.expect("smoke entry").path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "dart"))
        .collect();
    smoke_files.sort();
    assert!(!smoke_files.is_empty(), "no smoke tests in {smoke_dir:?}");
    for file in &smoke_files {
        let name = file.file_name().expect("file name");
        std::fs::copy(file, test_dir.join(name))
            .unwrap_or_else(|e| panic!("copy {}: {e}", file.display()));
    }
    eprintln!(
        "[flutter-widget-tests] wrote {} generated files + {} smoke tests to {}",
        result.files.len(),
        smoke_files.len(),
        app_dir.display()
    );

    // ── 3. Runtime gates (require a Flutter SDK) ──
    let Some(_flutter) = flutter_command() else {
        eprintln!(
            "[flutter-widget-tests] flutter SDK not found on PATH — \
             skipping analyze + flutter test (codegen gate still holds; \
             CI codegen-validate job runs this with flutter installed)"
        );
        return;
    };

    // 3a. pub get (idempotent; needed before analyze/test resolves). A
    // failure here is toolchain/network trouble (pub.dev must resolve
    // flutter_riverpod), not a codegen regression — degrade to SKIP,
    // mirroring the generated_code_validation sibling gate (I1).
    let pubget = flutter_run(&app_dir, &["pub", "get"]);
    if !pubget.status.success() {
        eprintln!(
            "[flutter-widget-tests] flutter pub get failed — is pub.dev \
             reachable? (flutter_riverpod must resolve)\n{}",
            output_text(&pubget)
        );
        return;
    }

    // 3b. dart-analyze gate — `flutter analyze` must stay clean on the
    // full generated project plus the smoke tests.
    let analyze = flutter_run(&app_dir, &["analyze"]);
    let analyze_out = output_text(&analyze);
    assert!(
        analyze.status.success(),
        "flutter analyze failed:\n{analyze_out}"
    );
    eprintln!("[flutter-widget-tests] flutter analyze: clean");

    // 3c. widget-test smoke suite (opt-in).
    if std::env::var("OMNILANG_FLUTTER_TEST").as_deref() != Ok("1") {
        eprintln!(
            "[flutter-widget-tests] OMNILANG_FLUTTER_TEST != 1 — skipping \
             `flutter test` (opt-in; run with OMNILANG_FLUTTER_TEST=1)"
        );
        return;
    }
    let tests = flutter_run(&app_dir, &["test"]);
    let test_out = output_text(&tests);
    assert!(tests.status.success(), "flutter test failed:\n{test_out}");
    eprintln!("[flutter-widget-tests] flutter test: PASS");
}
