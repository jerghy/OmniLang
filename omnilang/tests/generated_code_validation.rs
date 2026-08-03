//! Validate that GENERATED code actually compiles with real toolchains.
//!
//! String assertions in snapshot tests can pass while the emitted code fails
//! to type-check. These tests compile the semantic-DSL fixtures through the
//! full pipeline and run the real toolchains over the output:
//!
//! - `react_output_compiles_with_tsc`: fixtures -> React TSX -> `tsc --noEmit`
//! - `flutter_output_compiles_with_dart_analyze`: fixtures -> Dart -> `dart analyze`
//! - `match_variant_arms_flutter_compiles`: the showcase project
//!   (`test/projects/learn-omniview`) -> Flutter -> scoped `dart analyze`
//!   of `MatchBreadthDemo.dart` asserting deferral #30's specific errors
//!   (`Undefined class 'Rating'` / `Undefined name 'level'`) are gone.
//! - `learn_omniview_flutter_output_passes_dart_analyze`: the showcase
//!   project -> Flutter -> full-output `dart analyze` (still `#[ignore]`d —
//!   deferral #30's match root cause is fixed by Task 1.4, but the gate
//!   stays RED on other pre-existing gaps — Curves easing names, duplicated
//!   motion controllers, stdlib method emission, String->Color props — until
//!   the Phase 5 Flutter batch (Tasks 5.5/5.6/5.7) makes the full output
//!   analyze clean; that is the un-ignore point)
//!
//! Toolchain requirements (none are hard requirements — each test SKIPs
//! gracefully when its toolchain is missing, mirroring the pattern in
//! `js_runtime_tests.rs` / `omniview_runtime_smoke.rs`, so a machine without
//! node/dart never breaks `cargo test --workspace`):
//!
//! - React: `node` + `tsc` on PATH, or a local install of the TS packages
//!   under `tests/node_modules` (gitignored; the `codegen-validate` CI job
//!   runs `cd tests && npm init -y && npm install react@^19 zustand@^5
//!   typescript@5.5.4 @types/react@^19 && rm package.json package-lock.json`
//!   there — matching the harness app's react ^19.2.8 / zustand ^5.0.14,
//!   T0.4 upgrade).
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

/// Showcase project directory (relative to the omnilang crate root): the
/// flagship demo compiled through the real pipeline, exercising real
/// feature combinations (match / @for / theme / typed slots) that the
/// synthetic fixtures above do not cover (compilability review MEDIUM-10).
const LEARN_OMNIVIEW_PROJECT: &str = "../test/projects/learn-omniview";

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
             Install them once with: cd tests && npm init -y && npm install react@^19 \
             zustand@^5 typescript@5.5.4 @types/react@^19 && rm package.json package-lock.json \
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
  # Task 5.8: the stdlib registry emits `http.*` / `SharedPreferences`
  # for the Flutter target — the generated files import these packages.
  http: ^1.2.0
  shared_preferences: ^2.2.0
  intl: ^0.19.0
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

// ════════════════════════════════════════════════════════════════
// Showcase project: learn-omniview — full-project Flutter gate
// ════════════════════════════════════════════════════════════════
//
// The fixture gates above validate synthetic inputs; the flagship demo
// project exercises real combinations the fixtures do not cover (match /
// @for key / theme / typed slots — compilability review MEDIUM-10). The
// full-project gate compiles the WHOLE project to Flutter and runs
// `dart analyze` over EVERY generated `.dart` file; the scoped match test
// below analyzes a single file. Both share `compile_learn_omniview_flutter`
// (the project's output is written under `out/learn-omniview/` so
// `flutter_analyze_targets` picks it up unchanged).

/// Compile the showcase project to Flutter and write every generated `.dart`
/// file under `out/learn-omniview/` plus the pubspec/analysis_options
/// `dart analyze` needs. Returns the out root, or None when the Flutter
/// backend is not compiled into this build (SKIP).
fn compile_learn_omniview_flutter(scratch: &ScratchDir) -> Option<PathBuf> {
    // Discover sources the same way `omnic build` does: Directory mode
    // keys files by path relative to src/ ("./components/x" imports
    // resolve through the same path mapping the CLI uses).
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(LEARN_OMNIVIEW_PROJECT);
    assert!(
        project_root.is_dir(),
        "showcase project not found at {} — expected the learn-omniview \
         demo under test/projects/",
        project_root.display()
    );
    let sources = omnilang::discovery::discover_omni_sources(
        &project_root.join("src"),
        omnilang::discovery::DiscoveryMode::Directory,
    )
    .expect("discover learn-omniview sources");
    assert!(
        !sources.is_empty(),
        "no .omni sources found under {}",
        project_root.join("src").display()
    );

    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);
    let result = omnilang::compile_multi_internal(
        sources,
        Target::Flutter,
        path_mapping,
        false,
        false,
        None,
    );
    if backend_not_enabled(&result) {
        eprintln!(
            "SKIP: backend for Flutter not compiled into this build \
             (run with `--features codegen-flutter`); cannot validate \
             learn-omniview output"
        );
        return None;
    }
    // The showcase project demonstrates the DOM-only pointer-capture ref
    // ops (design D3). On Flutter those are a DELIBERATE hard error
    // ("Flutter 不支持指针捕获，用 GestureDetector" — spec §10.3), so the
    // project can never compile cleanly to Flutter while ElementRefDemo
    // keeps its pointer-capture section. Filter that expected diagnostic
    // out; any other error is a regression.
    let unexpected: Vec<String> = result
        .errors
        .iter()
        .filter(|d| !d.message.contains("Flutter 不支持指针捕获"))
        .map(|d| d.message.clone())
        .collect();
    assert!(
        unexpected.is_empty(),
        "learn-omniview compilation to Flutter failed: {:?}",
        unexpected
    );

    // Write every generated .dart file under out/learn-omniview/ (the same
    // out/<fixture>/ layout the fixture gate uses) so the shared
    // `flutter_analyze_targets` enumeration covers the project unchanged.
    let out_root = scratch.path().join("out");
    let out_dir = out_root.join("learn-omniview");
    std::fs::create_dir_all(&out_dir).expect("create project out dir");
    let mut written = 0usize;
    for (path, content) in &result.files {
        if !path.ends_with(".dart") {
            continue;
        }
        let dest = out_dir.join(Path::new(path).file_name().expect("output file name"));
        std::fs::write(&dest, content).expect("write generated file");
        written += 1;
    }
    assert!(
        written > 0,
        "no .dart files generated for learn-omniview ({written} written)"
    );

    std::fs::write(out_root.join("pubspec.yaml"), FLUTTER_PUBSPEC).expect("write pubspec");
    std::fs::write(out_root.join("analysis_options.yaml"), "# minimal\n").expect("write options");
    Some(out_root)
}

/// Scoped validation for deferral #30 (Task 1.4): match expressions with
/// variant arms must emit complete, compilable types. The showcase project's
/// `MatchBreadthDemo` uses `type Rating = Star { level: i32 } | Blank` —
/// the Flutter backend previously emitted `switch (r._variant)` / `Rating.*`
/// literals without ever emitting the `Rating` class or binding the variant
/// fields, so `dart analyze` reported `Undefined class 'Rating'`,
/// `Undefined name 'level'`, and `return_of_invalid_type`.
///
/// The fix: (1) union classes are emitted into a shared `types.dart` and
/// imported by the components that reference them; (2) variant match arms
/// bind their fields (`final level = _v.level;`); (3) matches without a
/// wildcard arm get a `default: throw StateError(...)` fallback so the
/// match IIFE's return type is not nullable.
#[test]
fn match_variant_arms_flutter_compiles() {
    let scratch = ScratchDir::new("match-variant");
    let Some(out_root) = compile_learn_omniview_flutter(&scratch) else {
        return; // backend not compiled in
    };

    // Deterministic content assertions — the emitted shape itself (these do
    // not depend on the dart toolchain being present).
    let demo_path = out_root
        .join("learn-omniview")
        .join("MatchBreadthDemo.dart");
    let types_path = out_root.join("learn-omniview").join("types.dart");
    let demo = std::fs::read_to_string(&demo_path).expect("MatchBreadthDemo.dart generated");
    let types = std::fs::read_to_string(&types_path).expect("types.dart generated");
    assert!(
        types.contains("class Rating"),
        "union class Rating must be emitted into types.dart:\n{types}"
    );
    assert!(
        types.contains("Rating.Star({ required this.level })"),
        "variant constructor must take the field as a named param \
         (VariantLiteral emits Rating.Star(level: ...)):\n{types}"
    );
    assert!(
        demo.contains("import './types.dart';"),
        "component using Rating must import types.dart:\n{demo}"
    );
    assert!(
        demo.contains("final level = _v.level;"),
        "variant arm must bind its fields from the subject:\n{demo}"
    );
    assert!(
        demo.contains("switch (_v.variant)"),
        "variant match keeps variant dispatch on the bound subject \
         (public member — the union class lives in the types.dart library):\n{demo}"
    );
    assert!(
        demo.contains("default: throw StateError"),
        "non-exhaustive match gets a throwing default so the IIFE return \
         type is non-nullable (kills return_of_invalid_type):\n{demo}"
    );

    // Analyzer gate — the MatchBreadthDemo-specific errors must be gone from
    // dart analyze's output for that file. Other pre-existing errors in the
    // file (Curves easing names, duplicated motion controllers, String->Color
    // props) belong to Phase 5 and are out of scope here.
    let Some(mut dart) = dart_command() else {
        eprintln!("SKIP: dart not found on PATH");
        return;
    };
    dart.args(["pub", "get"]).current_dir(&out_root);
    let pub_get = run(&mut dart);
    if !pub_get.status.success() {
        print_output("dart pub get", &pub_get);
        eprintln!(
            "SKIP: `dart pub get` failed — is dart the Flutter-bundled SDK, \
             and is pub.dev reachable? (flutter_riverpod must resolve)"
        );
        return;
    }

    let mut dart = dart_command().expect("dart still available");
    dart.args(["analyze"])
        .current_dir(&out_root)
        .arg("learn-omniview/MatchBreadthDemo.dart");
    let analyze = run(&mut dart);
    let stdout = String::from_utf8_lossy(&analyze.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&analyze.stderr).into_owned();
    let combined = format!("{stdout}{stderr}");
    for needle in [
        "Undefined class 'Rating'",
        "Undefined name 'Rating'",
        "Undefined name 'level'",
        "return_of_invalid_type",
    ] {
        assert!(
            !combined.contains(needle),
            "MatchBreadthDemo.dart must not report `{needle}` after the \
             deferral #30 fix. Full analyzer output:\n{combined}"
        );
    }
    eprintln!(
        "MatchBreadthDemo.dart: no deferral-#30 errors (Rating class / level \
         binding / return type). Residual Phase-5 errors in the file (Curves \
         easing names, duplicated motion controllers, String->Color props) \
         may remain:\n{combined}"
    );
}

// Task 5.8 cleared the showcase gate classes (333 -> 2 analyzer errors):
// Dart-family method mapping (`.len()`/`.to_string()`/`.push()`/...),
// cross-file lib-fn inlining, event wiring (constructor params, widget
// callbacks, tear-off handlers), store rewrite in fn/computed/effect bodies,
// inline stores, struct/union emission into types.dart, nullable types,
// measure-position parity, stdlib-driven imports (dart:math/convert, http,
// shared_preferences, intl), material-name hiding, string-concat coercion,
// await hoisting, fn renames for Dart type shadowing. The gate stays RED on
// TWO residual language-semantics gaps (adjudicated — defer-with-trigger):
// (1) untyped closure params (DataTable's `|s| s + 5` — the IR carries no
// closure param types, so Dart infers dynamic -> num vs int), and
// (2) a u64-max literal (TypeSystemDemo's 18446744073709551615 — beyond
// Dart's signed 64-bit int; BigInt would change the fn type).
#[test]
#[ignore = "learn-omniview Flutter output has 2 residual analyzer errors (untyped closure param -> num/int in DataTable; u64-max literal beyond Dart int in TypeSystemDemo) — adjudicated by the controller (see task-5.8 report)"]
fn learn_omniview_flutter_output_passes_dart_analyze() {
    let Some(mut dart) = dart_command() else {
        eprintln!("SKIP: dart not found on PATH");
        return;
    };

    let scratch = ScratchDir::new("learn-omniview");
    let Some(out_root) = compile_learn_omniview_flutter(&scratch) else {
        return; // backend not compiled in
    };

    dart.args(["pub", "get"]).current_dir(&out_root);
    let pub_get = run(&mut dart);
    if !pub_get.status.success() {
        print_output("dart pub get", &pub_get);
        eprintln!(
            "SKIP: `dart pub get` failed — is dart the Flutter-bundled SDK, \
             and is pub.dev reachable? (flutter_riverpod must resolve)"
        );
        return;
    }

    // EVERY generated .dart file of the showcase project must analyze
    // clean (warnings tolerated — unmapped semantic-DSL features degrade
    // to warnings per ADR-052/053).
    let targets = flutter_analyze_targets(&out_root);
    assert!(
        !targets.is_empty(),
        "no generated .dart files found under {} — the compile-level \
         validation above should have produced some",
        out_root.display()
    );
    let mut failures = Vec::new();
    for path in targets {
        if let Err(e) = analyze_dart(&out_root, &path) {
            failures.push(e);
        } else {
            eprintln!("dart analyze: OK ({path})");
        }
    }
    assert!(
        failures.is_empty(),
        "learn-omniview Flutter output failed `dart analyze`:\n{}",
        failures.join("\n\n")
    );
}

// ════════════════════════════════════════════════════════════════
// Showcase project: FormPage demo a11y migration (design doc 域 10)
// ════════════════════════════════════════════════════════════════
//
// FormPage.omni's 8 raw `aria-label` attributes migrated to
// `a11y: { label: ... }` blocks (Task 4.5). The contract: OmniView keeps
// emitting the aria-label attributes (no behavior change), React emits
// aria-label (with NO duplicate placeholder auto-fill label), and Flutter
// emits the Semantics(label: ...) node it used to silently drop (the
// review's MEDIUM-2).

/// Compile the whole learn-omniview project to a target, returning the
/// generated files keyed by output path. Returns None when the backend is
/// not compiled into this build (SKIP).
fn compile_learn_omniview(target: Target) -> Option<FxHashMap<String, String>> {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(LEARN_OMNIVIEW_PROJECT);
    let sources = omnilang::discovery::discover_omni_sources(
        &project_root.join("src"),
        omnilang::discovery::DiscoveryMode::Directory,
    )
    .expect("discover learn-omniview sources");
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);
    let result =
        omnilang::compile_multi_internal(sources, target, path_mapping, false, false, None);
    if result
        .errors
        .iter()
        .any(|d| d.message.contains(BACKEND_NOT_ENABLED))
    {
        eprintln!(
            "SKIP: backend for {target:?} not compiled into this build \
             (run with `--features codegen-flutter` for Flutter); \
             cannot validate learn-omniview output"
        );
        return None;
    }
    // The showcase project demonstrates the DOM-only pointer-capture ref
    // ops (design D3). On Flutter those are a DELIBERATE hard error
    // ("Flutter 不支持指针捕获，用 GestureDetector" — spec §10.3), so the
    // project can never compile cleanly to Flutter while ElementRefDemo
    // keeps its pointer-capture section. Filter that expected diagnostic
    // out; any other error is a regression.
    let unexpected: Vec<String> = result
        .errors
        .iter()
        .filter(|d| !d.message.contains("Flutter 不支持指针捕获"))
        .map(|d| d.message.clone())
        .collect();
    assert!(
        unexpected.is_empty(),
        "learn-omniview compilation to {target:?} failed: {:?}",
        unexpected
    );
    Some(result.files)
}

#[test]
fn learn_omniview_formpage_a11y_migration_across_targets() {
    // ── OmniView: the migrated a11y blocks must keep emitting the exact
    // aria-label attributes the raw attributes used to produce.
    if let Some(files) = compile_learn_omniview(Target::OmniView) {
        let body = files
            .get("body.html")
            .expect("omniview body.html generated");
        for label in [
            "Full name input",
            "Email address input",
            "Message input",
            "Disabled input",
            "Read-only input",
            "Submit form",
            "Reset form",
            "Conditional submit button",
        ] {
            assert!(
                body.contains(&format!("aria-label=\"{label}\"")),
                "OmniView must keep emitting aria-label=\"{label}\" after the \
                 a11y-block migration; got:\n{body}"
            );
        }
    }

    // ── React: aria-label present, and the placeholder auto-fill must NOT
    // add a duplicate aria-label (the a11y label suppresses the fallback).
    if let Some(files) = compile_learn_omniview(Target::React) {
        let form = files
            .get("components/FormPage.tsx")
            .expect("FormPage.tsx generated");
        assert!(
            form.contains("aria-label=\"Full name input\""),
            "React must emit the a11y-block label; got:\n{form}"
        );
        assert!(
            !form.contains("aria-label=\"Your name\""),
            "placeholder auto-fill must not duplicate the a11y label; got:\n{form}"
        );
        // Count: exactly one aria-label per field — the a11y label only.
        let count = form.matches("aria-label=").count();
        assert_eq!(
            count, 8,
            "React FormPage must emit exactly the 8 a11y labels; got {count}:\n{form}"
        );
    }

    // ── Flutter: the a11y intent is no longer silently dropped — every
    // migrated label becomes a Semantics(label: ...) node.
    if let Some(files) = compile_learn_omniview(Target::Flutter) {
        let form = files.get("FormPage.dart").expect("FormPage.dart generated");
        for label in [
            "Full name input",
            "Email address input",
            "Message input",
            "Disabled input",
            "Read-only input",
            "Submit form",
            "Reset form",
            "Conditional submit button",
        ] {
            assert!(
                form.contains(&format!("Semantics(label: '{label}'")),
                "Flutter must emit Semantics(label: '{label}') for the \
                 migrated a11y block (no silent drop); got:\n{form}"
            );
        }
    }
}
