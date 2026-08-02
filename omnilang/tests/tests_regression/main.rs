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
        content.contains("(() => {") && content.contains("const x = 1"),
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

    // Extract `([...items]).sort(...)` from `setItems(EXPR);`. The
    // expression starts at `([...items])` (whose own close paren is NOT the
    // end), so scan from the `sort(` call's open paren to its match.
    let start = out
        .find("([...items]).sort(")
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
