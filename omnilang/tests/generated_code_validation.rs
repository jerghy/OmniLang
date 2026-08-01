//! Validate that GENERATED code actually compiles with real toolchains.
//!
//! String assertions in snapshot tests can pass while the emitted code fails
//! to type-check. These tests compile the semantic-DSL fixtures through the
//! full pipeline and run the real toolchains over the output:
//!
//! - `react_output_compiles_with_tsc`: fixtures -> React TSX -> `tsc --noEmit`
//! - `flutter_output_compiles_with_dart_analyze`: fixtures -> Dart -> `dart analyze`
//!
//! Toolchain requirements (none are hard requirements — each test SKIPs
//! gracefully when its toolchain is missing, mirroring the pattern in
//! `js_runtime_tests.rs` / `omniview_runtime_smoke.rs`, so a machine without
//! node/dart never breaks `cargo test --workspace`):
//!
//! - React: `node` + `tsc` on PATH, or a local install of the TS packages
//!   under `tests/node_modules` (gitignored; the `codegen-validate` CI job
//!   runs `cd tests && npm init -y && npm install react@18.3.1 zustand@4.5.0
//!   typescript@5.5.4 @types/react@18.3.0 && rm package.json package-lock.json`
//!   there).
//!   The temp scratch dir lives under `tests/` so node's module resolution
//!   can walk up to `tests/node_modules`.
//! - Flutter: a Flutter-bundled `dart` (needed for the `sdk: flutter`
//!   dependency) plus network access to pub.dev so `dart pub get` can
//!   resolve `flutter_riverpod`; failures there degrade to SKIP.
//!
//! The Flutter backend is compiled only with `--features codegen-flutter`
//! (integration tests cannot set features). Without the feature the compiler
//! reports "Backend for 'Flutter' is not enabled" — detected at runtime and
//! reported as a SKIP.
//!
//! Flutter note: every generated `.dart` file of every parameterized
//! fixture is expected to pass `dart analyze` — the 6 known codegen bugs
//! that previously made widget output non-compiling (multi-node roots,
//! missing required `onPressed`, `Text(child: ...)` slot fallbacks,
//! unguarded nulls in `List<Widget>`, `(e)` closures on VoidCallback
//! params, non-String exprs in `Text()`) are fixed (deferral #17). New
//! fixtures are picked up automatically by the per-file enumeration.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use omnilang::codegen::Target;
use rustc_hash::FxHashMap;

/// Fixture directories (relative to the omnilang crate root).
const PARAMETERIZED_DIR: &str = "tests/fixtures/parameterized";
const REACT_MOTION_DIR: &str = "tests/fixtures/react";

/// Compiler error emitted when the crate was built without the target's
/// codegen-* feature (e.g. Flutter on a default-feature build).
const BACKEND_NOT_ENABLED: &str = "is not enabled";

/// Scratch root under `tests/` so node module resolution can walk up to
/// `tests/node_modules`; removed on drop (leftovers from crashed runs are
/// cleaned at construction).
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join(format!(
                "tmp_codegen_validation_{label}_{}",
                std::process::id()
            ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        ScratchDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// All `.omni` fixture files in a directory, sorted.
fn fixtures_in(dir: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("fixture dir {dir} should exist: {e}"))
        .map(|e| e.expect("read dir entry").path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "omni"))
        .collect();
    paths.sort();
    paths
}

/// React fixtures: all parameterized fixtures + the motion_* fixtures.
fn react_fixtures() -> Vec<PathBuf> {
    let mut fixtures = fixtures_in(PARAMETERIZED_DIR);
    fixtures.extend(fixtures_in(REACT_MOTION_DIR).into_iter().filter(|p| {
        p.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("motion_"))
    }));
    fixtures
}

/// Compile a single fixture to a target through the public compiler API.
fn compile_fixture(fixture: &Path, target: Target) -> omnilang::InternalCompileResult {
    let name = fixture
        .file_name()
        .expect("fixture file name")
        .to_string_lossy()
        .into_owned();
    let source = std::fs::read_to_string(fixture).expect("read fixture source");
    let mut sources = FxHashMap::default();
    sources.insert(name, source);
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);
    omnilang::compile_multi_internal(sources, target, path_mapping, false, false, None)
}

/// True when the compile failed because the target backend was not compiled
/// into the crate (missing codegen-* feature) rather than a real error.
fn backend_not_enabled(result: &omnilang::InternalCompileResult) -> bool {
    result
        .errors
        .iter()
        .any(|d| d.message.contains(BACKEND_NOT_ENABLED))
}

/// Compile every fixture to `target`, write the generated files (filtered to
/// `extensions`) into `out_dir/<fixture-name>/`, and return the fixture list.
/// SKIPs (returns `None`) when the target backend is not compiled in.
fn generate_fixtures(
    fixtures: &[PathBuf],
    target: Target,
    scratch: &ScratchDir,
    extensions: &[&str],
) -> Option<()> {
    for fixture in fixtures {
        let result = compile_fixture(fixture, target);
        if backend_not_enabled(&result) {
            eprintln!(
                "SKIP: backend for {target:?} not compiled into this build \
                 (run with `--features codegen-flutter` for Flutter); \
                 cannot validate generated output"
            );
            return None;
        }
        assert!(
            result.success,
            "compilation to {target:?} failed for {:?}: {:?}",
            fixture, result.errors
        );
        let out_dir = scratch
            .path()
            .join("out")
            .join(fixture.file_stem().expect("fixture stem"));
        std::fs::create_dir_all(&out_dir).expect("create per-fixture out dir");
        let mut written = 0usize;
        for (path, content) in &result.files {
            let matches = extensions.iter().any(|ext| path.ends_with(ext));
            if !matches {
                continue;
            }
            let dest = out_dir.join(Path::new(path).file_name().expect("output file name"));
            std::fs::write(&dest, content).expect("write generated file");
            written += 1;
        }
        assert!(
            written > 0,
            "no {extensions:?} files generated for {fixture:?}"
        );
    }
    Some(())
}

fn run(cmd: &mut Command) -> Output {
    cmd.output().expect("failed to spawn process")
}

fn print_output(label: &str, out: &Output) {
    eprintln!(
        "--- {label} stdout ---\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    eprintln!(
        "--- {label} stderr ---\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ════════════════════════════════════════════════════════════════
// React: tsc --noEmit over generated TSX
// ════════════════════════════════════════════════════════════════

/// Matches the committed tsconfig of `test/projects/learn-omniview/build/
/// react-test-app` (the e2e-validated generated app).
const REACT_TSCONFIG: &str = r#"{
  "compilerOptions": {
    "jsx": "react-jsx",
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": false,
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["**/*.ts", "**/*.tsx"]
}
"#;

/// Can node resolve the packages the generated code imports from `dir`?
/// Mirrors tsc's node_modules walk-up resolution closely enough for the
/// skip-vs-run decision.
fn deps_resolvable(dir: &Path) -> bool {
    let probe = "try { require.resolve('react/package.json'); \
                 require.resolve('zustand/package.json'); \
                 require.resolve('@types/react/package.json'); process.exit(0); } \
                 catch (e) { console.error(e.message); process.exit(1); }";
    Command::new("node")
        .args(["-e", probe])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Locate a usable `tsc`, returning (program, argv-without-`--noEmit`):
/// 1. `tsc` on PATH (global installs).
/// 2. A local `typescript` install resolved the same way the generated code
///    resolves `react` (node_modules walk-up from the output dir), executed
///    via `node`. This avoids npx/.bat shims, which Rust's CreateProcess
///    lookup cannot find on Windows (npm ships `tsc.cmd`, not `tsc.exe`).
fn find_tsc(dir: &Path) -> Option<(String, Vec<String>)> {
    if Command::new("tsc")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return Some(("tsc".to_string(), Vec::new()));
    }
    let resolve = Command::new("node")
        .args([
            "-e",
            "process.stdout.write(require.resolve('typescript/bin/tsc'))",
        ])
        .current_dir(dir)
        .output();
    let resolve = match resolve {
        Ok(out) if out.status.success() => out,
        _ => return None,
    };
    if resolve.status.success() {
        let tsc_js = String::from_utf8_lossy(&resolve.stdout).trim().to_string();
        if !tsc_js.is_empty() {
            return Some(("node".to_string(), vec![tsc_js]));
        }
    }
    None
}

/// A `Command` for the Flutter-bundled `dart`, or None when unavailable.
/// The Windows Flutter SDK ships `dart.bat` with no `dart.exe`, which Rust's
/// program lookup misses — fall back to `cmd.exe`, which honours PATHEXT.
fn dart_command() -> Option<Command> {
    if Command::new("dart")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return Some(Command::new("dart"));
    }
    #[cfg(windows)]
    if Command::new("cmd")
        .args(["/C", "dart", "--version"])
        .output()
        .is_ok_and(|o| o.status.success())
    {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("dart");
        return Some(cmd);
    }
    None
}

#[test]
fn react_output_compiles_with_tsc() {
    let fixtures = react_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {PARAMETERIZED_DIR} / {REACT_MOTION_DIR}"
    );

    let scratch = ScratchDir::new("react");
    if generate_fixtures(&fixtures, Target::React, &scratch, &["ts", "tsx", "css"]).is_none() {
        return; // backend not compiled in
    }
    let out_dir = scratch.path().join("out");
    std::fs::write(out_dir.join("tsconfig.json"), REACT_TSCONFIG).expect("write tsconfig");
    // The generated TSX imports `*.module.css` (emitted next to each
    // component). Real Vite projects satisfy these imports via
    // `vite/client` types; this minimal declaration mirrors that contract
    // so `tsc --noEmit` can resolve them.
    std::fs::write(
        out_dir.join("css-modules.d.ts"),
        "declare module '*.module.css' {\n  const classes: Record<string, string>;\n  export default classes;\n}\n",
    )
    .expect("write css-modules declaration");

    if !deps_resolvable(&out_dir) {
        eprintln!(
            "SKIP: react/zustand/@types/react not resolvable from generated output. \
             Install them once with: cd tests && npm init -y && npm install react@18.3.1 \
             zustand@4.5.0 typescript@5.5.4 @types/react@18.3.0 && rm package.json package-lock.json \
             (CI does this in the codegen-validate job)"
        );
        return;
    }
    let Some((program, mut argv)) = find_tsc(&out_dir) else {
        eprintln!("SKIP: tsc not found on PATH (and no local tests/node_modules install)");
        return;
    };

    argv.push("--noEmit".to_string());
    let mut cmd = Command::new(program);
    cmd.args(&argv).current_dir(&out_dir);
    let out = run(&mut cmd);
    if !out.status.success() {
        print_output("tsc --noEmit", &out);
        panic!(
            "generated React TSX failed `tsc --noEmit` ({} fixtures compiled): \
             see output above",
            fixtures.len()
        );
    }
    eprintln!(
        "tsc --noEmit: OK ({} fixtures compiled clean in {})",
        fixtures.len(),
        out_dir.display()
    );
}

// ════════════════════════════════════════════════════════════════
// Flutter: dart analyze over generated Dart
// ════════════════════════════════════════════════════════════════
//
// `dart analyze` exit codes (Dart 3.x): 0 = no issues, 2 = warnings only,
// 3+ = errors. Unmapped semantic-DSL features degrade to WARNINGS by design
// (ADR-052/053), so the test fails only on analyzer ERRORS and reports
// warnings as a note.

const FLUTTER_PUBSPEC: &str = r#"name: omnilang_generated_validation
environment:
  sdk: ^3.5.0
dependencies:
  flutter:
    sdk: flutter
  flutter_riverpod: ^3.0.0
"#;

/// Collect every generated `.dart` file under `out_dir/<fixture>/` as
/// `"<fixture>/<file>"` paths, sorted. This is the full analyze target
/// list — ALL generated output is expected to pass `dart analyze`
/// (deferral #17 fixed the 6 known widget-codegen bugs; new fixtures are
/// covered automatically).
fn flutter_analyze_targets(out_dir: &Path) -> Vec<String> {
    let mut targets: Vec<String> = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(out_dir)
        .expect("out dir")
        .map(|e| e.expect("read out dir entry").path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();
    for dir in entries {
        let fixture = dir
            .file_name()
            .expect("fixture dir name")
            .to_string_lossy()
            .into_owned();
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .expect("fixture out dir")
            .map(|e| e.expect("read fixture out dir entry").path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "dart"))
            .collect();
        files.sort();
        for file in files {
            let name = file
                .file_name()
                .expect("output file name")
                .to_string_lossy()
                .into_owned();
            targets.push(format!("{fixture}/{name}"));
        }
    }
    targets
}

/// Analyze `path` (relative to `out_dir`) with `dart analyze`.
/// Returns `Ok(())` when no analyzer errors are reported; warnings-only
/// results (exit code 2) pass with a printed note, since unmapped semantic
/// DSL features degrade to warnings by design (ADR-052/053).
fn analyze_dart(out_dir: &Path, path: &str) -> Result<(), String> {
    let mut dart = dart_command().expect("dart still available");
    dart.args(["analyze"]).current_dir(out_dir).arg(path);
    let analyze = run(&mut dart);
    let stdout = String::from_utf8_lossy(&analyze.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&analyze.stderr).into_owned();
    let combined = format!("{stdout}{stderr}");

    let has_errors = combined
        .lines()
        .any(|l| l.contains(" error - ") || l.contains(" error • "));
    let warning_count = combined
        .lines()
        .filter(|l| l.contains(" warning - ") || l.contains(" warning • "))
        .count();

    if has_errors {
        return Err(format!("analyzer ERRORS in {path}:\n{combined}"));
    }
    if !analyze.status.success() {
        if warning_count > 0 {
            eprintln!(
                "NOTE: dart analyze reported {warning_count} warning(s) only \
                 (unmapped design features degrade to warnings per ADR-052/053):\n{combined}"
            );
        } else {
            return Err(format!(
                "dart analyze failed unexpectedly for {path}:\n{combined}"
            ));
        }
    }
    Ok(())
}

#[test]
fn flutter_output_compiles_with_dart_analyze() {
    let Some(mut dart) = dart_command() else {
        eprintln!("SKIP: dart not found on PATH");
        return;
    };

    let fixtures = fixtures_in(PARAMETERIZED_DIR);
    assert!(
        !fixtures.is_empty(),
        "no fixtures under {PARAMETERIZED_DIR}"
    );

    let scratch = ScratchDir::new("flutter");
    if generate_fixtures(&fixtures, Target::Flutter, &scratch, &["dart"]).is_none() {
        return; // backend not compiled in
    }
    let out_dir = scratch.path().join("out");
    std::fs::write(out_dir.join("pubspec.yaml"), FLUTTER_PUBSPEC).expect("write pubspec");
    // Keep analyzer defaults; no lint package is declared so this only pins
    // the file's existence for determinism.
    std::fs::write(out_dir.join("analysis_options.yaml"), "# minimal\n").expect("write options");

    // `dart pub get` needs the Flutter SDK (sdk: flutter dep) and pub.dev
    // access to resolve flutter_riverpod. If either is missing the test
    // degrades to SKIP rather than failing on toolchain setup.
    dart.args(["pub", "get"]).current_dir(&out_dir);
    let pub_get = run(&mut dart);
    if !pub_get.status.success() {
        print_output("dart pub get", &pub_get);
        eprintln!(
            "SKIP: `dart pub get` failed — is dart the Flutter-bundled SDK, \
             and is pub.dev reachable? (flutter_riverpod must resolve)"
        );
        return;
    }

    // EVERY generated .dart file must analyze clean (warnings tolerated —
    // unmapped semantic-DSL features degrade to warnings per ADR-052/053).
    let targets = flutter_analyze_targets(&out_dir);
    assert!(
        !targets.is_empty(),
        "no generated .dart files found under {} — the compile-level \
         validation above should have produced some",
        out_dir.display()
    );
    let mut failures = Vec::new();
    for path in targets {
        if let Err(e) = analyze_dart(&out_dir, &path) {
            failures.push(e);
        } else {
            eprintln!("dart analyze: OK ({path})");
        }
    }
    assert!(
        failures.is_empty(),
        "generated Flutter Dart failed `dart analyze`:\n{}",
        failures.join("\n\n")
    );
}
