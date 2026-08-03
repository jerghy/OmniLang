//! Cross-backend regression tests — inline-style expression channel (域 8,
//! Task 1.3, review finding C2 root cause).
//!
//! The style channel (`style: { prop: expr }`) lowers expressions to JSON
//! markers (`$signal` / `$field` / `$call` / `$concat`) consumed by each
//! backend. Before the fix, ANY `obj.field` in a style expression was
//! mangled to `$signal: "obj_field"` — correct only for genuine store
//! signal aliases (`ThemeStore.currentBg` → `ThemeStore_currentBg`); for
//! loop-item fields like `row.score` it produced the undefined identifier
//! `row_score` (verified in generated React AND OmniView output).
//!
//! Contract under test:
//!   - `obj.field` where obj is a store alias  → `$signal: "obj_field"` (unchanged)
//!   - `obj.field` where obj is a signal       → `$field` with `$signal` obj
//!     (React emits `obj.field`, OmniView emits `obj.value.field`)
//!   - `obj.field` where obj is a local        → `$field` with `$ident` obj
//!     (both targets emit `obj.field`)

use rustc_hash::FxHashMap;

use omnilang::codegen::{compile, Target};
use omnilang::ir::lower;
use omnilang::lexer::tokenize;
use omnilang::parser;

/// Minimal C2 fixture: `@for` item field read inside a style expression.
/// Mirrors learn-omniview DataTable.omni (`getScoreColor(row.score)`).
const FOR_ITEM_FIELD_FIXTURE: &str = r##"
import omniview;

struct RowItem {
    id: i32,
    score: i32,
}

component App {
    signal rows: Array<RowItem> = [];

    fn getScoreColor(score: i32) -> String {
        if score >= 90 { return "#16a34a"; }
        return "#dc2626";
    }

    view {
        <div>
            @for row in rows {
                <span style: { color: getScoreColor(row.score) }>{row.score}</span>
            }
        </div>
    }
}
"##;

/// Genuine store alias fixture: `PaletteStore.color` must keep the
/// `$signal: "PaletteStore_color"` channel (store fields are real signals
/// in every backend — NOT real JS field access).
const STORE_FIELD_FIXTURE: &str = r##"
import omniview;

store PaletteStore {
    signal color: String = "#ff0000";
}

component App {
    @store PaletteStore;

    view {
        <div style: { color: PaletteStore.color }></div>
    }
}
"##;

/// Signal-object fixture: `user.name` where `user` is a signal holding an
/// object — must be REAL field access on the signal's value, not a mangled
/// `user_name` alias.
const SIGNAL_OBJECT_FIELD_FIXTURE: &str = r##"
import omniview;

component App {
    signal user: Map<String, String> = { name: "Ada", theme: "#0f0" };

    view {
        <div style: { color: user.theme }></div>
    }
}
"##;

// ── Task 1.5: statement-bearing BlockExpr must hard-diagnose on
//    python/vue (previously silently dropped), and `array_sort` with a
//    two-arg comparator must actually sort (review M1: "Toggle Sort"
//    produced a permanent no-op). ─────────────────────────────────

/// Fixture: a statement-bearing block expression in expression position.
/// Python and Vue cannot emit statements inside expressions — they must
/// report an E_CODEGEN error instead of silently dropping them (React
/// supports them via IIFE and compiles clean).
const BLOCK_EXPR_STMT_FIXTURE: &str = r##"
import omniview;

component BlockExprStmt {
    signal result: i32 = 0;

    fn compute() {
        let y = { let x = 1; x + 1 };
        result = y;
    }

    view {
        <div>
            <button @onPress={compute}>go</button>
        </div>
    }
}
"##;

/// Fixture: the review's M1 repro — `array_sort` with a two-param
/// (boolean) comparator. Must emit a real comparator and actually reorder.
const SORT_TWO_ARG_COMPARATOR_FIXTURE: &str = r##"
import omniview;
import { array_sort } from "std/array";

struct Item { name: string }

component SortM1 {
    signal items: Array<Item> = [];

    fn sortIt() { items = array_sort(items, |a, b| a.name > b.name); }

    view {
        <div>
            <button @onPress={sortIt}>sort</button>
        </div>
    }
}
"##;

/// Compile without asserting error-freedom — returns the raw CompileOutput
/// so tests can assert on the diagnostics.
fn compile_source_output(source: &str, target: Target) -> omnilang::codegen::CompileOutput {
    let tokens = tokenize(source);
    let (program, parse_errors) = parser::parse(tokens, source);
    assert!(
        parse_errors.is_empty(),
        "parse errors: {:?}",
        parse_errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
    let mut semantics = omnilang::semantic::analyze(&program);
    assert!(
        semantics.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        semantics
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
    let ir = lower(&program, &mut semantics, None);
    let source_files: FxHashMap<String, String> =
        FxHashMap::from_iter([("input.omni".to_string(), source.to_string())]);
    compile(&ir, target, &source_files)
}

/// Statement-bearing BlockExpr on Python → E_CODEGEN error, not a silent drop.
#[cfg(feature = "codegen-python")]
#[test]
fn python_block_expr_statements_diagnosed_not_dropped() {
    let output = compile_source_output(BLOCK_EXPR_STMT_FIXTURE, Target::Python);
    assert!(
        !output.errors.is_empty(),
        "statement-bearing block expressions must hard-diagnose on Python — \
         previously the statements were silently dropped"
    );
    let e = &output.errors[0];
    assert_eq!(
        e.kind,
        omnilang::error::DiagnosticKind::ECodegen,
        "expected an E_CODEGEN diagnostic, got: {:?}",
        e
    );
    assert!(
        e.message.contains("block expression"),
        "diagnostic must explain the block expression, got: {}",
        e.message
    );
}

/// Statement-bearing BlockExpr on Vue → E_CODEGEN error, not a silent drop.
#[cfg(feature = "codegen-vue")]
#[test]
fn vue_block_expr_statements_diagnosed_not_dropped() {
    let output = compile_source_output(BLOCK_EXPR_STMT_FIXTURE, Target::Vue);
    assert!(
        !output.errors.is_empty(),
        "statement-bearing block expressions must hard-diagnose on Vue — \
         previously the statements were silently dropped"
    );
    let e = &output.errors[0];
    assert_eq!(
        e.kind,
        omnilang::error::DiagnosticKind::ECodegen,
        "expected an E_CODEGEN diagnostic, got: {:?}",
        e
    );
    assert!(
        e.message.contains("block expression"),
        "diagnostic must explain the block expression, got: {}",
        e.message
    );
}

/// Positive control: React supports statement-bearing block expressions
/// (IIFE) — the same fixture compiles clean there.
#[test]
fn react_block_expr_statements_compile_clean() {
    let output = compile_source_output(BLOCK_EXPR_STMT_FIXTURE, Target::React);
    assert!(
        output.errors.is_empty(),
        "React must compile statement-bearing block expressions cleanly: {:?}",
        output
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
    let content = output
        .files
        .iter()
        .map(|f| f.content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("(() => {") && content.contains("let x = 1"),
        "React must emit the block statements inside an IIFE. Got:\n{}",
        content
    );
}

/// The review's M1 repro: a two-arg comparator must be emitted as a real
/// comparator and must actually reorder the array when executed in node.
#[cfg(feature = "codegen-react")]
#[test]
fn node_array_sort_two_arg_comparator_sorts() {
    // Skip if Node.js is not available (mirrors tests/js_runtime_tests.rs)
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("SKIP: Node.js not found -- skipping node array_sort test");
        return;
    }

    let out = compile_react(SORT_TWO_ARG_COMPARATOR_FIXTURE);
    // The two-param closure must be emitted through the comparator path
    // (`const $r=` with a numeric fast-path), NOT the $ka/$kb machinery.
    assert!(
        out.contains("const $r=") && !out.contains("const $ka="),
        "two-arg closure must be emitted as a real comparator. Got:\n{}",
        out
    );

    // Extract `([...i]).sort(...)` from the functional-updater emission
    // `setItems((i) => ([...i]).sort(...))` (Task 3.5 M2: the mutation
    // reads the signal it writes, so the value goes through the updater
    // param). The expression starts at `([...i])` (whose own close paren is
    // NOT the end), so scan from the `sort(` call's open paren to its match.
    let start = out
        .find("([...i]).sort(")
        .unwrap_or_else(|| panic!("no sort expression in React output:\n{}", out));
    let sort_paren = out[start..]
        .find("sort(")
        .map(|i| start + i + 4)
        .expect("sort( marker");
    let mut depth = 0i32;
    let mut end = sort_paren;
    for (i, c) in out[sort_paren..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = sort_paren + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let expr = &out[start..end];

    let script = format!(
        r##"
const items = [{{ name: "b" }}, {{ name: "a" }}, {{ name: "c" }}, {{ name: "f" }}, {{ name: "e" }}, {{ name: "d" }}];
const i = items; // the M2 functional-updater param bound to the array
const sorted = ({expr}).map(i => i.name);
const expected = ["a", "b", "c", "d", "e", "f"];
console.log("SORTED=" + JSON.stringify(sorted));
if (JSON.stringify(sorted) !== JSON.stringify(expected)) {{
    throw new Error("array_sort with a two-arg comparator did not sort: " + JSON.stringify(sorted));
}}
"##
    );
    let mut child = std::process::Command::new("node")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn node");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let output = child.wait_with_output().expect("node output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "node array_sort eval failed:\nstdout: {}\nstderr: {}\nexpr: {}",
        stdout,
        stderr,
        expr
    );
}

/// Compile a source string to a target, returning all generated file
/// contents concatenated. Panics on parse/compile errors.
fn compile_source_all_files(source: &str, target: Target) -> String {
    let tokens = tokenize(source);
    let (program, parse_errors) = parser::parse(tokens, source);
    assert!(
        parse_errors.is_empty(),
        "parse errors: {:?}",
        parse_errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
    // Mirror the real pipeline (run_pipeline): semantic analysis populates
    // the symbol table BEFORE lowering — the style-channel lowering consults
    // it to distinguish store aliases from plain locals.
    let mut semantics = omnilang::semantic::analyze(&program);
    assert!(
        semantics.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        semantics
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
    let ir = lower(&program, &mut semantics, None);
    let source_files: FxHashMap<String, String> =
        FxHashMap::from_iter([("input.omni".to_string(), source.to_string())]);
    let output = compile(&ir, target, &source_files);
    assert!(
        output.errors.is_empty(),
        "compile errors: {:?}",
        output
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
    output
        .files
        .iter()
        .map(|f| f.content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_react(source: &str) -> String {
    compile_source_all_files(source, Target::React)
}

#[cfg(feature = "codegen-omniview")]
fn compile_omniview(source: &str) -> omnilang::OmniViewResult {
    omnilang::compile_omniview(source, false, false)
}

// ── @for item field: must NOT be mangled to `row_score` ─────

#[test]
fn react_style_expr_for_item_field_not_mangled() {
    let out = compile_react(FOR_ITEM_FIELD_FIXTURE);
    assert!(
        out.contains("getScoreColor(row.score)"),
        "React style must emit the real field access `getScoreColor(row.score)`. Got:\n{}",
        out
    );
    assert!(
        !out.contains("row_score"),
        "React style must NOT mangle `row.score` into `row_score`. Got:\n{}",
        out
    );
}

#[cfg(feature = "codegen-omniview")]
#[test]
fn omniview_style_expr_for_item_field_not_mangled() {
    let result = compile_omniview(FOR_ITEM_FIELD_FIXTURE);
    assert!(
        result.success,
        "OmniView compile failed: {:?}",
        result.errors
    );
    let js = result.files.expect("files").js;
    assert!(
        js.contains("row.score"),
        "OmniView JS must emit the real field access `row.score`. Got:\n{}",
        js
    );
    assert!(
        !js.contains("row_score"),
        "OmniView JS must NOT mangle `row.score` into `row_score`. Got:\n{}",
        js
    );
}

// ── Genuine store alias: `$signal` channel unchanged ────────

#[test]
fn react_style_expr_store_field_keeps_signal_channel() {
    let out = compile_react(STORE_FIELD_FIXTURE);
    assert!(
        out.contains("PaletteStore_color"),
        "React style must keep the store-field signal alias `PaletteStore_color`. Got:\n{}",
        out
    );
    assert!(
        !out.contains("PaletteStore.color"),
        "React style must NOT emit `PaletteStore.color` (store field is a signal, not JS field access). Got:\n{}",
        out
    );
}

#[cfg(feature = "codegen-omniview")]
#[test]
fn omniview_style_expr_store_field_keeps_signal_channel() {
    let result = compile_omniview(STORE_FIELD_FIXTURE);
    assert!(
        result.success,
        "OmniView compile failed: {:?}",
        result.errors
    );
    let js = result.files.expect("files").js;
    assert!(
        js.contains("PaletteStore_color.value"),
        "OmniView style must keep the store-field signal read `.value`. Got:\n{}",
        js
    );
}

// ── Signal-object field: real field access on the signal value ─

#[cfg(feature = "codegen-omniview")]
#[test]
fn omniview_style_expr_signal_object_field_resolves_value() {
    let result = compile_omniview(SIGNAL_OBJECT_FIELD_FIXTURE);
    assert!(
        result.success,
        "OmniView compile failed: {:?}",
        result.errors
    );
    let js = result.files.expect("files").js;
    assert!(
        js.contains("user.value.theme"),
        "OmniView style must read the field on the signal's value (`user.value.theme`). Got:\n{}",
        js
    );
    assert!(
        !js.contains("user_theme"),
        "OmniView style must NOT mangle `user.theme` into `user_theme`. Got:\n{}",
        js
    );
}

// ── Runtime: the style value must resolve correctly in node ──

/// Extract the style-binding expression from the OmniView JS output and
/// evaluate it in node with a sample `row`, proving the emitted field
/// access computes the right CSS value (no `row_score` ReferenceError).
#[cfg(feature = "codegen-omniview")]
#[test]
fn node_style_expr_field_value_resolves_at_runtime() {
    // Skip if Node.js is not available (mirrors tests/js_runtime_tests.rs)
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("SKIP: Node.js not found -- skipping node style-value test");
        return;
    }

    let result = compile_omniview(FOR_ITEM_FIELD_FIXTURE);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let js = result.files.expect("files").js;

    // The pre-compiled per-clone style binding emits:
    //   _el.style.color = __omni_sanitizeCss(<fn>(row.score));
    // Extract the inner expression. Match the ASSIGNMENT form
    // `=__omni_sanitizeCss(` so the runtime function definition
    // `function __omni_sanitizeCss(value) {` is not picked up.
    let marker = "=__omni_sanitizeCss(";
    let Some(open) = js.find(marker) else {
        panic!("no style binding assignment in OmniView JS:\n{}", js);
    };
    let rest = &js[open + marker.len()..];
    let close = rest.find(')').expect("closing paren");
    let expr = &rest[..=close];
    assert!(
        expr.contains("row.score"),
        "extracted style expression must read row.score, got: {}",
        expr
    );

    // Stub the component fn (prefixed, e.g. App_getScoreColor) so the
    // extracted expression evaluates in isolation.
    let fn_name = expr
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    assert!(
        !fn_name.is_empty()
            && fn_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "could not extract fn name from style expression: {}",
        expr
    );
    let script = format!(
        r##"
const __omni_sanitizeCss = (v) => v;
const {fn_name} = (score) => score >= 90 ? "#16a34a" : "#dc2626";
const row = {{ score: 95 }};
const color = ({}).trim();
console.log("STYLE_VALUE=" + color);
if (color !== "#16a34a") {{ throw new Error("expected #16a34a for score 95, got " + color); }}
"##,
        expr,
        fn_name = fn_name
    );
    let mut child = std::process::Command::new("node")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn node");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let output = child.wait_with_output().expect("node output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "node style-value eval failed:\nstdout: {}\nstderr: {}\nexpr: {}",
        stdout,
        stderr,
        expr
    );
}

// ── Task 1.6 (C1 + TS batch): functions called from interpolations / props /
//    inline styles must be EMITTED; fn params and useState get TS types ────
//
// Before the fix, `collect_called_fns_from_view` only scanned event handlers
// and class_map_bindings — DataTable.omni's interpolation calls
// (getSortIcon / getScoreColor / getActiveRows / getGradeForScore /
// getBonusTotal) were never declared → TS2304 ×6 and ReferenceError at
// render.

/// DataTable-like fixture: component fns called from an interpolation, a
/// plain prop value, an inline style (`$call` channel), and a @for item
/// style — ALL must be declared in the React output.
const C1_VIEW_CALLED_FN_FIXTURE: &str = r##"
import omniview;

struct RowItem {
    id: i32,
    score: i32,
}

component App {
    signal rows: Array<RowItem> = [];
    signal title: String = "";

    fn getScoreColor(score: i32) -> String {
        if score >= 90 { return "#16a34a"; }
        return "#dc2626";
    }

    fn getSortIcon() -> String { return "ASC"; }

    fn getActiveRows() -> Array<RowItem> { return rows; }

    fn getTitle() -> String { return title; }

    view {
        <div>
            <p>Sort: {getSortIcon()}</p>
            <img src={getTitle()} />
            <span style: { color: getScoreColor(95) }>x</span>
            @for row in rows {
                <span style: { color: getScoreColor(row.score) }>{row.score}</span>
            }
            <p>Active: {(getActiveRows()).length}</p>
        </div>
    }
}
"##;

#[test]
fn interpolation_called_fns_are_emitted() {
    let out = compile_react(C1_VIEW_CALLED_FN_FIXTURE);
    for f in ["getSortIcon", "getScoreColor", "getActiveRows", "getTitle"] {
        assert!(
            out.contains(&format!("const {} =", f)),
            "interpolation/prop/style-called fn `{f}` must be declared. Got:\n{}",
            out
        );
    }
    assert!(
        out.contains("getSortIcon()"),
        "interpolation call must be emitted. Got:\n{}",
        out
    );
    assert!(
        out.contains("getScoreColor(row.score)"),
        "style-channel field access must be emitted. Got:\n{}",
        out
    );
    assert!(
        out.contains("src={getTitle()}"),
        "prop-value call must be emitted. Got:\n{}",
        out
    );
}

/// A component-local fn passed by VALUE into a prop (`transform={shout}`)
/// must be emitted — the call walker previously only saw CallExpr, so the
/// identifier reference was never collected (TS2304 on `shout`).
const FN_REF_PROP_FIXTURE: &str = r##"
import omniview;

component FnFormatter {
    prop transform: fn(String) -> String = null;

    view {
        <p>{transform("hi")}</p>
    }
}

component App {
    fn shout(s: String) -> String { return s; }

    view {
        <div>
            <FnFormatter transform={shout} />
        </div>
    }
}
"##;

#[test]
fn fn_referenced_by_value_in_prop_is_emitted() {
    let out = compile_react(FN_REF_PROP_FIXTURE);
    assert!(
        out.contains("const shout = (s: string) => {"),
        "fn passed as a prop value must be declared with typed params. Got:\n{}",
        out
    );
    assert!(
        out.contains("transform={shout}"),
        "prop value reference must be emitted. Got:\n{}",
        out
    );
}

/// Module-level pub fns calling other pub fns: the callee must be emitted
/// too (previously only the view-referenced entry was declared → TS2304 on
/// the inner call).
const LIB_FN_TRANSITIVE_FIXTURE: &str = r##"
import omniview;

pub fn inner(x: String) -> String { return x; }

pub fn outer(x: String) -> String { return inner(x); }

component App {
    view {
        <p>{outer("hi")}</p>
    }
}
"##;

#[test]
fn lib_fn_transitive_callees_are_emitted() {
    let out = compile_react(LIB_FN_TRANSITIVE_FIXTURE);
    assert!(
        out.contains("const outer = (x: string) => {"),
        "entry lib fn must be declared with typed params. Got:\n{}",
        out
    );
    assert!(
        out.contains("const inner = (x: string) => {"),
        "transitively-called lib fn must be declared. Got:\n{}",
        out
    );
}

/// TS batch: fn params typed from FunctionIR param types, and
/// `useState<T>` typed initialization for resolvable (builtin) element
/// types — fixes the never[]-cascade (TS2322/TS2345) for array state.
const TYPED_FN_PARAM_AND_STATE_FIXTURE: &str = r##"
import omniview;

component App {
    signal items: Array<String> = [];
    signal count: i32 = 0;

    fn greet(name: String) -> String { return "hi " + name; }

    fn reset() {
        items = ["a", "b"];
        count = 1;
    }

    view {
        <div>
            <p>{greet("Ada")}</p>
            <p>{items}</p>
            <p>{count}</p>
            <button @onPress={reset}>reset</button>
        </div>
    }
}
"##;

#[test]
fn fn_params_and_use_state_are_typed() {
    let out = compile_react(TYPED_FN_PARAM_AND_STATE_FIXTURE);
    assert!(
        out.contains("const greet = (name: string) => {"),
        "fn params must carry TS types from the IR. Got:\n{}",
        out
    );
    assert!(
        out.contains("useState<string[]>([])"),
        "array state must emit a typed initializer (never[] cascade fix). Got:\n{}",
        out
    );
    assert!(
        out.contains("useState<number>(0)"),
        "primitive state must emit a typed initializer. Got:\n{}",
        out
    );
}

/// A signal read ONLY from a class map binding (`class: { success: x }`)
/// must still get useState + setter — previously it was emitted as a plain
/// const, so the generated `setSubmitted(...)` was TS2304.
const CLASS_MAP_READ_SIGNAL_FIXTURE: &str = r##"
import omniview;

component App {
    signal submitted: bool = false;

    fn mark() { submitted = true; }

    view {
        <div>
            <p class: { status: true, success: submitted }>x</p>
            <button @onPress={mark}>go</button>
        </div>
    }
}
"##;

#[test]
fn class_map_binding_reads_emit_state_hook() {
    let out = compile_react(CLASS_MAP_READ_SIGNAL_FIXTURE);
    assert!(
        out.contains("const [submitted, setSubmitted] = useState<boolean>(false);"),
        "signal read only via class map binding must get useState + setter. Got:\n{}",
        out
    );
}

/// bind:value-driven setters must count as mutations — the bound signal
/// needs useState + setter (previously `const x = ""` + an undefined
/// `setInputText` call → TS2304).
const BINDING_MUTATION_FIXTURE: &str = r##"
import omniview;

component App {
    signal inputText: String = "";

    view {
        <input bind:value={inputText} />
    }
}
"##;

#[test]
fn binding_driven_setters_emit_state_hook() {
    let out = compile_react(BINDING_MUTATION_FIXTURE);
    assert!(
        out.contains("const [inputText, setInputText] = useState<string>(\"\");"),
        "bind:value signal must get useState + setter. Got:\n{}",
        out
    );
}

// ── Task 3.4: @effect deps 语义验证 + 闭包依赖盲区修复 ─────────
//
// (A) `getter_expr_contains_ident` (React computed deps) never descended into
//     ClosureExpr — FilterBar's `filteredFrameworks` reads `searchText` only
//     inside `|f| ...` and emitted `useMemo(..., [demoFrameworks])`; typing
//     never re-filtered. The closure arm fixes it (perf HIGH-3).
// (B) `validate_effect_deps` (omnilang-core) — declared @effect(deps) must
//     cover every store-field read in the effect body (including closure-
//     internal reads), else E_EFFECT_DEPS_MISSING compile error (the
//     NotificationsPanel class of silent bug).

/// The task fixture: three components — one missing-dep (must error), two
/// full-coverage (must pass). Loading it through the full pipeline must
/// produce EXACTLY one diagnostic: the missing NotificationStore.unreadCount.
const EFFECT_DEPS_FIXTURE: &str = include_str!("../fixtures/effect_deps.omni");

/// Missing-dep variant where the unread store field is read ONLY inside a
/// closure — the perf HIGH-3 blind spot at the effect level.
const EFFECT_DEPS_CLOSURE_MISSING_FIXTURE: &str = r##"
import omniview;
import { array_filter } from "std/array";

store NotificationStore {
    signal notifications: Array<String> = [];
    getter unreadCount: i32 = notifications.len();
}

component App {
    @store NotificationStore;
    signal label: Array<String> = [];

    @effect(notifications) {
        let check = array_filter(NotificationStore.notifications, |n| n == "x" || NotificationStore.unreadCount > 0);
        label = check;
    }

    view {
        <div>{label.len()}</div>
    }
}
"##;

/// Full-coverage variant of the same fixture — closure-internal read declared.
const EFFECT_DEPS_CLOSURE_COVERED_FIXTURE: &str = r##"
import omniview;
import { array_filter } from "std/array";

store NotificationStore {
    signal notifications: Array<String> = [];
    getter unreadCount: i32 = notifications.len();
}

component App {
    @store NotificationStore;
    signal label: Array<String> = [];

    @effect(NotificationStore.unreadCount, NotificationStore.notifications) {
        let check = array_filter(NotificationStore.notifications, |n| n == "x" || NotificationStore.unreadCount > 0);
        label = check;
    }

    view {
        <div>{label.len()}</div>
    }
}
"##;

/// The perf HIGH-3 repro: a computed whose only read of `searchText` is
/// inside a pipe closure — the React useMemo deps MUST include searchText.
const COMPUTED_CLOSURE_READ_FIXTURE: &str = r##"
import omniview;
import { array_filter } from "std/array";

component App {
    signal searchText: String = "";
    signal demoFrameworks: Array<String> = ["React", "Vue"];

    computed filteredFrameworks: Array<String> = array_filter(demoFrameworks, |f| f == searchText);

    view {
        <div>{filteredFrameworks}</div>
    }
}
"##;

/// Run lex → parse → semantic → lower and return the diagnostics emitted
/// DURING lowering (where validate_effect_deps runs) — the real pipeline
/// merges these into the compile output.
fn lower_source_diagnostics(source: &str) -> Vec<omnilang::error::Diagnostic> {
    let tokens = tokenize(source);
    let (program, parse_errors) = parser::parse(tokens, source);
    assert!(
        parse_errors.is_empty(),
        "parse errors: {:?}",
        parse_errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
    let mut semantics = omnilang::semantic::analyze(&program);
    assert!(
        semantics.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        semantics
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
    let _ir = lower(&program, &mut semantics, None);
    std::mem::take(&mut semantics.diagnostics)
}

#[test]
fn effect_deps_fixture_missing_dep_is_compile_error() {
    let diags = lower_source_diagnostics(EFFECT_DEPS_FIXTURE);
    let errors: Vec<&omnilang::error::Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == omnilang::error::Severity::Error)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "the bad component must produce exactly one error (the two good \
         components must stay clean), got: {:?}",
        errors.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
    let e = errors[0];
    assert_eq!(
        e.kind,
        omnilang::error::DiagnosticKind::EEffectDepsMissing,
        "expected E_EFFECT_DEPS_MISSING, got: {:?}",
        e.kind
    );
    assert_eq!(
        e.message, "effect 依赖漏掉：NotificationStore.unreadCount",
        "verbatim message required, got: {}",
        e.message
    );
    assert!(
        e.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("NotificationStore.unreadCount")),
        "suggestion must name the missing dep, got: {:?}",
        e.suggestion
    );
}

#[test]
fn effect_deps_closure_internal_read_missing_errors() {
    // perf HIGH-3 at the effect level: the read of unreadCount happens only
    // inside the `|n| ...` closure — it must still require a dep.
    let diags = lower_source_diagnostics(EFFECT_DEPS_CLOSURE_MISSING_FIXTURE);
    let messages: Vec<&str> = diags
        .iter()
        .filter(|d| d.severity == omnilang::error::Severity::Error)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        messages,
        vec!["effect 依赖漏掉：NotificationStore.unreadCount"],
        "closure-internal store read must be counted, got: {:?}",
        messages
    );
}

#[test]
fn effect_deps_closure_internal_read_covered_compiles_clean() {
    let diags = lower_source_diagnostics(EFFECT_DEPS_CLOSURE_COVERED_FIXTURE);
    assert!(
        diags.is_empty(),
        "full coverage (including the closure-internal read) must compile \
         clean, got: {:?}",
        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn computed_closure_signal_read_lands_in_memo_deps() {
    // perf HIGH-3 repro (FilterBar case): before the ClosureExpr arm in
    // getter_expr_contains_ident this emitted `}, [demoFrameworks]);` and
    // typing never re-filtered the computed.
    let out = compile_react(COMPUTED_CLOSURE_READ_FIXTURE);
    assert!(
        out.contains("}, [searchText, demoFrameworks]);"),
        "the closure-internal searchText read must be a useMemo dep. Got:\n{}",
        out
    );
}

/// Review F1 repro: a computed reading a component prop — the React
/// useMemo deps MUST include the prop name. Props were missing from the
/// known-names set used for computed deps, so the memo silently went stale
/// when the parent re-rendered with a new prop value (same bug class as
/// perf HIGH-3).
const COMPUTED_PROP_READ_FIXTURE: &str = r##"
import omniview;

component App {
    prop label: String = "";

    computed display: String = label + "!";

    view {
        <div>{display}</div>
    }
}
"##;

#[test]
fn computed_prop_read_lands_in_memo_deps() {
    let out = compile_react(COMPUTED_PROP_READ_FIXTURE);
    assert!(
        out.contains("}, [label]);"),
        "a computed reading a prop must list the prop in its useMemo deps. Got:\n{}",
        out
    );
}

// ── Task 3.5: store BNF 统一 + React mutation 质量批次 ─────────
//
// (A) 顶层与内联 store 成员集统一为 signal|getter|action（设计文档域 5
//     BNF v2）；`fn` 成员两层都是错误（「store 内 fn 成员不允许；用
//     getter 或 action」）。
// (B) M2: 同一 signal 自读的赋值（`count = count + 1`）发射函数式更新
//     （`setCount((c) => c + 1)`），消除批处理/StrictMode 下的闭包陈旧
//     读（评审 M2）。
// (C) M4: action 内连续 set() 合并为单次 set({ k1: v1, k2: v2 })（评审
//     M4——三次顺序 set 对一次逻辑更新产生三次订阅通知）。
// (D) M3: 空事件对象不再发射 `Object.assign(e, {})` 噪音；不再需要
//     事件数据时发射无参箭头 `() => {...}`（评审 M3）。

/// (A) The BNF fixture: clean stores (signal/getter/action at both levels)
/// must parse; `fn` members at both levels must produce the verbatim
/// diagnostic. Each bad block also emits one stray-brace recovery error —
/// filtered out by message matching.
const STORE_BNF_FIXTURE: &str = include_str!("../fixtures/store_bnf.omni");

#[test]
fn store_bnf_fixture_fn_members_error_at_both_levels() {
    let tokens = tokenize(STORE_BNF_FIXTURE);
    let (program, errors) = parser::parse(tokens, STORE_BNF_FIXTURE);
    let fn_errors: Vec<&omnilang::error::ParseError> = errors
        .iter()
        .filter(|e| e.message.contains("store 内 fn 成员不允许"))
        .collect();
    assert_eq!(
        fn_errors.len(),
        2,
        "top-level AND inline store fn members must each error, got: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    assert!(
        errors
            .iter()
            .all(|e| e.message.contains("store 内 fn 成员不允许")
                || e.message.contains("Unexpected token at top level")),
        "only the fn diagnostic + recovery noise expected, got: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );

    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.stores.len(), 1, "clean top-level store must parse");
    assert_eq!(module.stores[0].signals.len(), 1);
    assert_eq!(module.stores[0].getters.len(), 1);
    assert_eq!(module.stores[0].actions.len(), 1);
    assert_eq!(
        module.components.len(),
        1,
        "clean inline-store component must parse"
    );
    assert_eq!(
        module.components[0].signals.len(),
        1,
        "inline signal flattened"
    );
    assert_eq!(
        module.components[0].getters.len(),
        1,
        "inline getter flattened"
    );
    assert_eq!(
        module.components[0].actions.len(),
        1,
        "inline action flattened"
    );
}

/// (T4.1) themes.omni — v2 域 3/域 5 (D1 语法层) 主题系统 fixture：
/// `@themes { dark, light }` 声明 + `@theme` 信号标记，全管线编译后
/// DesignIR.theme_signal 必须被接线（wiring-ready IR shape）。
const THEMES_FIXTURE: &str = include_str!("../fixtures/themes.omni");

#[test]
fn themes_fixture_wires_theme_signal_through_pipeline() {
    let mut sources = FxHashMap::default();
    sources.insert("themes.omni".to_string(), THEMES_FIXTURE.to_string());
    let path_mapping = FxHashMap::default();
    let out = omnilang_core::analyse::analyse_multi(&sources, &path_mapping);
    assert!(out.success, "themes.omni must compile cleanly");
    assert!(
        out.diagnostics.is_empty(),
        "themes.omni must be diagnostic-free, got: {:?}",
        out.diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );

    let ir = out.ir.expect("should have IR");
    let module = ir.modules.get("input.omni").unwrap();
    let design = module
        .designs
        .iter()
        .find(|d| d.name == "AppDesign")
        .expect("should have AppDesign");
    assert_eq!(design.themes, vec!["dark", "light"]);
    assert_eq!(
        design.theme_signal.as_deref(),
        Some("ThemeStore.mode"),
        "the @theme-marked store signal must be wired into DesignIR.theme_signal"
    );
    assert_eq!(design.default_theme.as_deref(), Some("light"));
    let store = module
        .stores
        .get("ThemeStore")
        .expect("should have ThemeStore");
    assert_eq!(store.theme_signal.as_deref(), Some("mode"));
}

/// (B) Full-pipeline M2: `count = count + step` must emit the functional
/// updater; `count = 0` (no self-read) must keep the direct setter form.
const M2_SELF_READ_FIXTURE: &str = r##"
import omniview;

component Counter {
    signal count: i32 = 0;
    signal step: i32 = 1;

    fn inc() { count = count + step; }
    fn reset() { count = 0; }

    view {
        <div>
            {count}
            <button @onPress={inc}>+</button>
            <button @onPress={reset}>R</button>
        </div>
    }
}
"##;

#[test]
fn react_component_mutation_self_read_emits_functional_updater() {
    let out = compile_react(M2_SELF_READ_FIXTURE);
    assert!(
        out.contains("setCount((c) => c + step);"),
        "self-reading mutation must use the functional updater. Got:\n{}",
        out
    );
    assert!(
        out.contains("setCount(0);"),
        "non-self-reading mutation must keep the direct setter form. Got:\n{}",
        out
    );
    assert!(
        !out.contains("setCount(count + step)"),
        "the stale-closure direct form must not appear. Got:\n{}",
        out
    );
}

/// (C) Full-pipeline M4: three consecutive store mutations in one action
/// coalesce into ONE `set({ ... })` call.
const M4_COALESCE_FIXTURE: &str = r##"
import omniview;

store TodoStore {
    signal items: Array<String> = [];
    signal nextId: i32 = 0;
    signal lastAction: String = "none";
    signal filterStatus: String = "all";

    action add(text: String) {
        items = [...items, text];
        nextId = nextId + 1;
        lastAction = "added";
    }

    action setFilter(status: String) {
        filterStatus = status;
        lastAction = "filter: " + status;
    }
}
"##;

#[test]
fn react_store_action_consecutive_sets_coalesce_into_one() {
    let out = compile_react(M4_COALESCE_FIXTURE);
    let add_idx = out.find("add: (text) => {").expect("add action emitted");
    let add_end = out[add_idx..]
        .find(
            "
    },",
        )
        .expect("add action closing brace");
    let add_block = &out[add_idx..add_idx + add_end];
    let set_count = add_block.matches("set(").count();
    assert_eq!(
        set_count, 1,
        "three consecutive mutations must coalesce into ONE set() call, got:\n{}",
        add_block
    );
    assert!(
        add_block.contains("nextId: state.nextId + 1")
            && add_block.contains("lastAction: \"added\""),
        "coalesced set() must carry all three keys, got:\n{}",
        add_block
    );
}

/// (D) Full-pipeline M3: a handler whose body needs no event fields must
/// NOT emit `Object.assign(e, {})`, and the now-unused implicit event param
/// is dropped entirely — the handler becomes a paramless `() => {...}`.
const M3_EMPTY_EVENT_FIXTURE: &str = r##"
import omniview;

component App {
    signal log: String = "";

    view {
        <button @onPress={() => { log = "hi"; }}>Hi</button>
    }
}
"##;

#[test]
fn react_inline_handler_empty_event_skips_object_assign() {
    let out = compile_react(M3_EMPTY_EVENT_FIXTURE);
    assert!(
        !out.contains("Object.assign(e, {})"),
        "empty event objects must not emit Object.assign noise. Got:\n{}",
        out
    );
    assert!(
        out.contains("onClick={() => {"),
        "a handler needing no event data must emit a paramless arrow. Got:\n{}",
        out
    );
}

/// (D) Positive control: a handler that DOES read event fields keeps the
/// extraction and the plain `e` param.
const M3_EVENT_FIELDS_FIXTURE: &str = r##"
import omniview;

component App {
    signal lastX: f64 = 0.0;

    view {
        <button @onPress={(e) => { lastX = e.x; }}>X</button>
    }
}
"##;

#[test]
fn react_inline_handler_with_event_fields_keeps_extraction() {
    let out = compile_react(M3_EVENT_FIELDS_FIXTURE);
    assert!(
        out.contains("Object.assign(e, { x:") || out.contains("onClick={(e) => {"),
        "used event fields must keep the extraction path. Got:\n{}",
        out
    );
}
