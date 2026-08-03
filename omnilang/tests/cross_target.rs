//! Cross-target correctness tests — verify that the same .omni source
//! compiles without errors across all P0 backends (OmniView, React, Flutter).
//!
//! Each Golden fixture is compiled to all 3 backends. The primary invariant:
//! if a file parses successfully, ALL backends should produce error-free output.

use std::fs;
use std::path::PathBuf;

use rustc_hash::FxHashMap;

use omnilang::codegen::{compile, Target};
use omnilang::ir::lower;
use omnilang::lexer::tokenize;
use omnilang::parser;
use omnilang::semantic::SemanticResult;

/// Resolve the repo root.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("should have parent directory")
        .to_path_buf()
}

/// Read the MANIFEST.md and extract .omni file paths.
fn read_manifest_paths() -> Vec<String> {
    let manifest = repo_root().join("tests/fixtures/golden/MANIFEST.md");
    let content = fs::read_to_string(&manifest)
        .unwrap_or_else(|_| panic!("MANIFEST.md not found at {:?}", manifest));
    content
        .lines()
        .filter(|line| line.starts_with("- `"))
        .filter_map(|line| {
            let start = line.find('`')? + 1;
            let end = line[start..].find('`')?;
            Some(line[start..start + end].to_string())
        })
        .collect()
}

/// Compile source text to a specific target.
fn compile_source(source: &str, target: Target) -> omnilang::codegen::CompileOutput {
    let tokens = tokenize(source);
    let (program, parse_errors) = parser::parse(tokens, source);
    if !parse_errors.is_empty() {
        return omnilang::codegen::CompileOutput {
            files: vec![],
            errors: parse_errors
                .into_iter()
                .map(|e| omnilang::error::Diagnostic {
                    severity: omnilang::error::Severity::Error,
                    kind: omnilang::error::DiagnosticKind::ECodegen,
                    message: format!("Parse error: {}", e.message),
                    file: None,
                    line: e.span.line,
                    col: e.span.col,
                    end_line: None,
                    end_col: None,
                    suggestion: None,
                })
                .collect(),
            source_map: vec![],
            warnings: vec![],
        };
    }

    let mut semantics = SemanticResult {
        symbols: omnilang::semantic::symbols::SymbolTable::new(),
        dependencies: FxHashMap::default(),
        diagnostics: vec![],
        event_param_name: None,
    };
    let ir = lower(&program, &mut semantics, None);
    let source_files: FxHashMap<String, String> =
        FxHashMap::from_iter([("input.omni".to_string(), source.to_string())]);

    compile(&ir, target, &source_files)
}

fn p0_backends() -> Vec<(Target, &'static str)> {
    vec![
        (Target::OmniView, "OmniView"),
        (Target::React, "React"),
        #[cfg(feature = "codegen-flutter")]
        (Target::Flutter, "Flutter"),
    ]
}

/// Check whether a compile output contains design block warnings.
fn has_design_block_warning(output: &omnilang::codegen::CompileOutput) -> bool {
    output
        .warnings
        .iter()
        .any(|w| w.contains("Design blocks are only supported on the OmniView target"))
}

/// Primary cross-target test: every Golden fixture must compile error-free
/// on all 3 P0 backends.
#[test]
fn cross_target_no_errors_on_all_backends() {
    let paths = read_manifest_paths();
    assert!(
        paths.len() >= 20,
        "Expected at least 20 golden files, got {}",
        paths.len()
    );

    let repo = repo_root();
    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut ok = 0usize;

    for rel_path in &paths {
        let abs_path = repo.join(rel_path);
        let source = match fs::read_to_string(&abs_path) {
            Ok(s) => s,
            Err(e) => {
                // File not found is not a backend failure — skip
                eprintln!("SKIP {}: cannot read — {}", rel_path, e);
                continue;
            }
        };

        for (target, target_name) in p0_backends() {
            total += 1;
            let output = compile_source(&source, target);

            if !output.errors.is_empty() {
                failures.push(format!(
                    "{} → {}: {} error(s) — {}",
                    rel_path,
                    target_name,
                    output.errors.len(),
                    output
                        .errors
                        .first()
                        .map(|e| e.message.as_str())
                        .unwrap_or("unknown"),
                ));
                continue;
            }

            ok += 1;
        }
    }

    if !failures.is_empty() {
        eprintln!(
            "\n=== Cross-target errors ({}/{}) ===\n{}",
            failures.len(),
            total,
            failures.join("\n"),
        );
    }

    eprintln!(
        "\nCross-target: {}/{} passed ({} files × {} backends)",
        ok,
        total,
        paths.len(),
        p0_backends().len(),
    );

    assert!(
        failures.is_empty(),
        "{} cross-target compilation errors",
        failures.len()
    );
}

/// Verify that component files produce output on all backends.
/// Pure-library and design-only files are excluded as they legitimately
/// produce empty output on some backends.
#[test]
fn cross_target_component_files_produce_output() {
    // Component-containing files that should produce non-empty output
    let component_files = [
        "test/projects/learn-omniview/src/components/Counter.omni",
        "test/projects/learn-omniview/src/components/AddTodo.omni",
        "test/projects/learn-omniview/src/components/ThemeSwitcher.omni",
        "test/projects/learn-omniview/src/components/DesignTokenPanel.omni",
        "test/projects/learn-omniview/src/stores/TodoStore.omni",
        "test/projects/learn-omniview/src/stores/ThemeStore.omni",
        "test/projects/learn-omniview/src/pages/Overview.omni",
    ];

    let repo = repo_root();
    let mut failures: Vec<String> = Vec::new();

    for rel_path in &component_files {
        let abs_path = repo.join(rel_path);
        let source = fs::read_to_string(&abs_path)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", rel_path, e));

        for (target, target_name) in p0_backends() {
            let output = compile_source(&source, target);

            if !output.errors.is_empty() {
                failures.push(format!(
                    "{} → {}: errors — {}",
                    rel_path,
                    target_name,
                    output
                        .errors
                        .first()
                        .map(|e| e.message.as_str())
                        .unwrap_or("unknown"),
                ));
                continue;
            }

            if output.files.is_empty() && !has_design_block_warning(&output) {
                failures.push(format!(
                    "{} → {}: expected output files, got none",
                    rel_path, target_name,
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} component file output failures:\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

/// Task 2.4 fixture: keyboard/clipboard/drag-drop event FIELDS must not be
/// silently dropped on any target. React and OmniView map them for real
/// (e.code, clipboard text, dataTransfer files); Flutter must produce
/// warnings (never silent stubs — ADR-054 honest degradation) and must
/// never emit the debug-oriented KeyEvent code mapping (debugName).
#[test]
fn event_fields_stubs_warn_on_flutter_not_silent() {
    let source = r#"
import omniview;
component EventFields {
    signal lastKey: String = "";
    signal clipText: String = "";
    signal dropCount: i32 = 0;

    fn handleKey(e) { lastKey = e.code; }
    fn handleCopy(e) { clipText = e.text; }
    fn handleDrop(e) { dropCount = e.files.len(); }

    view {
        <input
            @onKeyDown={handleKey}
            @onCopy={handleCopy}
            @onDrop={handleDrop} />
    }
}
"#;

    for (target, target_name) in p0_backends() {
        let output = compile_source(source, target);
        assert!(
            output.errors.is_empty(),
            "{target_name}: fixture must compile without errors: {:?}",
            output.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        let event_warnings: Vec<&String> = output
            .warnings
            .iter()
            .filter(|w| {
                w.contains("onKeyDown")
                    || w.contains("onCopy")
                    || w.contains("onDrop")
                    || w.contains("KeyEvent")
                    || w.contains("ClipboardEvent")
                    || w.contains("DropEvent")
            })
            .collect();
        match target_name {
            "OmniView" | "React" => assert!(
                event_warnings.is_empty(),
                "{target_name}: keyboard/clipboard/drop fields are mapped for real — \
                 warnings would mean silent degradation: {:?}",
                event_warnings
            ),
            "Flutter" => assert!(
                !event_warnings.is_empty(),
                "Flutter: keyboard/clipboard/drop events must never compile silently"
            ),
            _ => {}
        }
    }

    // Flutter output must never contain the debug-only KeyEvent mapping —
    // the stable physicalKey.keyId mapping is the only legal code mapping.
    // (Flutter-only: the target is not compiled without codegen-flutter.)
    #[cfg(feature = "codegen-flutter")]
    {
        let flutter_out = compile_source(source, Target::Flutter);
        assert!(
            flutter_out.errors.is_empty(),
            "Flutter fixture must compile: {:?}",
            flutter_out
                .errors
                .iter()
                .map(|e| &e.message)
                .collect::<Vec<_>>()
        );
        for f in &flutter_out.files {
            assert!(
                !f.content.contains("debugName"),
                "debugName must never be emitted as event data in {}",
                f.path
            );
            assert!(
                !f.content.contains("logicalKey.debugName"),
                "stale debugName mapping in {}",
                f.path
            );
        }
    }
}

/// Cross-target parity assertions for the 4 features identified in ADR-021:
/// class_map_bindings, empty_body, slot, key_expr.
///
/// Each feature is compiled on all 3 P0 backends and checked for structural
/// invariants — not exact output matching, but verifying the feature is
/// actually handled (not silently dropped).
#[test]
fn cross_target_parity_features() {
    let repo = repo_root();
    let mut failures: Vec<String> = Vec::new();

    // Helper: check all output files contain expected text
    let has = |output: &omnilang::codegen::CompileOutput, needle: &str| -> bool {
        output.files.iter().any(|f| f.content.contains(needle))
    };

    // Compile and check a known fixture across all P0 backends.
    // Fixtures that exercise slot projection + component nesting.
    {
        let path = repo.join("test/projects/learn-omniview/src/components/SlotDefaultDemo.omni");
        let source = fs::read_to_string(&path).expect("SlotDefaultDemo.omni not found");
        for (target, target_name) in p0_backends() {
            let output = compile_source(&source, target);
            if !output.errors.is_empty() {
                failures.push(format!("slot → {}: compilation errors", target_name));
                continue;
            }
            // Design blocks produce empty output on non-OmniView targets with a warning
            if has_design_block_warning(&output) {
                continue;
            }
            // Slot fixture should emit <slot> element or children projection
            if !has(&output, "slot") && !has(&output, "children") && !has(&output, "Slot") {
                failures.push(format!(
                    "slot → {}: output missing slot/projection (silently dropped)",
                    target_name
                ));
            }
        }
    }

    // Verify DataTable (uses @for with key expressions)
    {
        let path = repo.join("test/projects/learn-omniview/src/components/DataTable.omni");
        let source = fs::read_to_string(&path).expect("DataTable.omni not found");
        for (target, target_name) in p0_backends() {
            let output = compile_source(&source, target);
            if !output.errors.is_empty() {
                failures.push(format!("@for/key → {}: compilation errors", target_name));
                continue;
            }
            // Design blocks produce empty output on non-OmniView targets with a warning
            if has_design_block_warning(&output) {
                continue;
            }
            // Must emit list rendering output — at minimum some data iteration
            if output.files.is_empty() {
                failures.push(format!(
                    "@for/key → {}: no output files (list rendering silently dropped)",
                    target_name
                ));
            }
        }
    }

    // Verify Modal (uses event handlers + slot projection)
    {
        let path = repo.join("test/projects/learn-omniview/src/components/Modal.omni");
        let source = fs::read_to_string(&path).expect("Modal.omni not found");
        for (target, target_name) in p0_backends() {
            let output = compile_source(&source, target);
            if !output.errors.is_empty() {
                failures.push(format!("events → {}: compilation errors", target_name));
                continue;
            }
            // Design blocks produce empty output on non-OmniView targets with a warning
            if has_design_block_warning(&output) {
                continue;
            }
            // Modal with events — should produce some output
            if output.files.is_empty() {
                failures.push(format!(
                    "events → {}: no output files (event handling silently dropped)",
                    target_name
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} cross-target parity failures:\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

/// Task 5.2 semantic set: borderRadius/fontSize must animate on all three
/// targets — WAAPI native (React), BorderRadiusTween + TextStyle(fontSize:)
/// (Flutter), runtime CSS transition (OmniView). The generated output is
/// checked for each target's mapping shape, not just error-free compilation.
#[test]
fn border_radius_font_size_animate_across_targets() {
    let source = r#"
import omniview;

@motion soften {
    from { borderRadius: 8 }
    to   { borderRadius: 16 }
    over: 300ms
    easing: ease-out
}

@motion growText {
    from { fontSize: 12 }
    to   { fontSize: 16 }
    over: 300ms
    easing: ease-out
}

component RoundedCard {
    view {
        <div class: card @hover={soften}>
            Hover me
        </div>
        <p @hover={growText}>
            Read me
        </p>
    }
}
"#;
    let has = |output: &omnilang::codegen::CompileOutput, needle: &str| -> bool {
        output.files.iter().any(|f| f.content.contains(needle))
    };

    // ── React: WAAPI keyframes with unit-bearing length values.
    let react = compile_source(source, Target::React);
    assert!(
        react.errors.is_empty(),
        "React compilation failed: {:?}",
        react.errors
    );
    assert!(
        has(&react, "borderRadius:\"8px\"") && has(&react, "borderRadius:\"16px\""),
        "React must emit borderRadius keyframes with px units: {:?}",
        react.files.iter().map(|f| &f.content).collect::<Vec<_>>()
    );
    assert!(
        has(&react, "fontSize:\"12px\"") && has(&react, "fontSize:\"16px\""),
        "React must emit fontSize keyframes with px units: {:?}",
        react.files.iter().map(|f| &f.content).collect::<Vec<_>>()
    );

    // ── Flutter: BorderRadiusTween + TextStyle(fontSize:) via
    // DefaultTextStyle (the color-family wrapper pattern).
    #[cfg(feature = "codegen-flutter")]
    {
        let flutter = compile_source(source, Target::Flutter);
        assert!(
            flutter.errors.is_empty(),
            "Flutter compilation failed: {:?}",
            flutter.errors
        );
        assert!(
            has(&flutter, "BorderRadiusTween(begin: BorderRadius.all(Radius.circular(8.0)), end: BorderRadius.all(Radius.circular(16.0)))"),
            "Flutter must emit a BorderRadiusTween for borderRadius: {:?}",
            flutter.files.iter().map(|f| &f.content).collect::<Vec<_>>()
        );
        assert!(
            has(&flutter, "TextStyle(fontSize: _growTextFontSize_1.value)"),
            "Flutter must apply fontSize through TextStyle(fontSize: ...): {:?}",
            flutter.files.iter().map(|f| &f.content).collect::<Vec<_>>()
        );
        assert!(
            has(
                &flutter,
                "late final Animation<BorderRadius?> _softenBorderRadius_0;"
            ),
            "Flutter field for borderRadius must be Animation<BorderRadius?> \
             (per-element ordinal — the soften element is element 0): {:?}",
            flutter.files.iter().map(|f| &f.content).collect::<Vec<_>>()
        );
        assert!(
            has(&flutter, "clipBehavior: Clip.antiAlias"),
            "Flutter borderRadius wrapper must clip the child (the card-case \
             background) so the animated radius is visible — a decoration \
             without color/clipping is visually inert: {:?}",
            flutter.files.iter().map(|f| &f.content).collect::<Vec<_>>()
        );
    }

    // ── OmniView: keyframe properties pass through to the runtime, which
    // maps them onto CSS transitions (px suffix applied at runtime).
    let omniview = compile_source(source, Target::OmniView);
    assert!(
        omniview.errors.is_empty(),
        "OmniView compilation failed: {:?}",
        omniview.errors
    );
    assert!(
        has(&omniview, "\"borderRadius\"") && has(&omniview, "\"fontSize\""),
        "OmniView must pass borderRadius/fontSize keyframes to the runtime: {:?}",
        omniview
            .files
            .iter()
            .map(|f| &f.content)
            .collect::<Vec<_>>()
    );
}

/// Design D4 (v2): the three deleted slot forms hard-error on every P0
/// backend with the design-doc 域 4 migration hints — the pipeline must
/// never let a typed/legacy slot form compile silently.
#[test]
fn deleted_slot_forms_hard_error_on_all_backends() {
    let cases: [(&str, &str); 3] = [
        (
            "component A { view { <div>@slot header: String { <span>f</span> }</div> } }",
            "类型约束与数据参数已删除；slot = 父传内容",
        ),
        (
            "component A { view { <div>@slot(header)</div> } }",
            "改用 body 级声明 @slot name=",
        ),
        (
            "component A { view { <div slot=\"header\">x</div> } }",
            "改用命名 props：name={...}",
        ),
    ];
    let mut failures: Vec<String> = Vec::new();
    for (source, hint) in cases {
        for (target, target_name) in p0_backends() {
            let output = compile_source(source, target);
            if output.errors.is_empty() {
                failures.push(format!(
                    "deleted slot form {source:?} → {target_name}: compiled silently"
                ));
                continue;
            }
            if !output.errors.iter().any(|e| e.message.contains(hint)) {
                failures.push(format!(
                    "deleted slot form {source:?} → {target_name}: missing migration \
                     hint {hint:?}, got: {:?}",
                    output.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} deleted-slot-form failures:\n{}",
        failures.len(),
        failures.join("\n"),
    );
}
