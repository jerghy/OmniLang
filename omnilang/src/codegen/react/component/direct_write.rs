//! Direct-write channel EMISSION (ADR-071 decision 2, Phase 1).
//!
//! The D1/D2/B2 signal-domain CLASSIFICATION is target-independent and
//! lives in `crate::codegen::shared::direct_write` (Task 12) — this module
//! holds only the React-target emission half: `useRef` storage cells,
//! mutation-site apply injection, and the B2 `useEffect([deps])` direct
//! writes. See the shared module doc for the classification contract.

use crate::codegen::shared::direct_write::{DirectWriteEntry, DirectWriteSets};
use crate::codegen::shared::style::{format_style_prop_key, format_style_value_expr};
use crate::ir::ComponentIR;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════
// D1/D2 storage emission (Phase 1, Task 2)
// ═══════════════════════════════════════════════════════════

/// Emit `const nameRef = useRef<T>(initial);` for D1/D2 signals.
///
/// D1 (direct-write) and D2 (write-only) signals move OFF the useState
/// path into a `useRef` cell: D1 reads resolve to `{name}Ref.current`
/// inside fn/effect bodies (always the live value — Task 3 emission
/// contract) and mutations inject a direct DOM write; D2 has no output
/// reads at all. The initializer and the TS generic reuse the exact
/// useState translation (`format_expr_ir_tsx` + `typed_state_annotation`,
/// the same gate state.rs applies — a type that does not resolve in the
/// generated file stays unannotated). Sorted by name for deterministic
/// output; a trailing blank line separates the block from the effects
/// that follow (mirrors gen_ref_hooks).
pub(super) fn gen_direct_write_storage(
    w: &mut crate::codegen::CodeWriter,
    comp: &ComponentIR,
    sets: &DirectWriteSets,
    user_types: &FxHashSet<String>,
) {
    let mut direct_sorted: Vec<String> = sets.d1.iter().chain(sets.d2.iter()).cloned().collect();
    direct_sorted.sort();
    if direct_sorted.is_empty() {
        return;
    }
    for name in &direct_sorted {
        let sig = comp.signals.iter().find(|s| s.name == *name).unwrap();
        let init = crate::codegen::react::expr::format_expr_ir_tsx(
            &sig.initial_value,
            &FxHashSet::default(),
        );
        let type_ann = super::state::typed_state_annotation(&sig.r#type, user_types);
        if type_ann.is_empty() {
            w.emit_fmt(format_args!("const {}Ref = useRef({});", name, init));
        } else {
            w.emit_fmt(format_args!(
                "const {}Ref = useRef<{}>({});",
                name, type_ann, init
            ));
        }
    }
    w.emit("");
}

// ═══════════════════════════════════════════════════════════
// px 单位注入（Task 15 gate round）
// ═══════════════════════════════════════════════════════════

/// CSS dimension properties whose NON-string values need a `px` unit when
/// written through the direct channels. React's JSX style prop appends px
/// to numbers automatically, but a raw `el.style.width = 80` write is
/// unit-less invalid CSS — the browser silently drops it. The set mirrors
/// the Flutter channel's mapped property set (`css_prop_to_flutter_param`:
/// width/height/padding/margin — the cross-target contract's dimension
/// surface; `--*` custom props never get a unit).
const DIMENSION_PROPS: [&str; 4] = ["width", "height", "padding", "margin"];

/// Whether a style entry VALUE is statically a CSS string (no `px` suffix
/// may be appended): a String literal, a sanitize-wrapped value
/// ($concat/$call/$field — `sanitizeCss` returns a string), or a
/// String-typed signal/computed read (declared-type lookup — mirrors
/// Flutter's `style_value_is_string`). Everything else (numeric literals,
/// numeric signals, unknown $ident reads) is treated as a number at
/// runtime and gets the `` `${...}px` `` template on dimension props. The
/// template is also what keeps the emitted code TS-clean: the DOM lib
/// types `style.{prop}` as `string` (TS2322 on a raw numeric write — the
/// regression the regenerated react harness exposed in Task 15).
fn entry_value_is_string(e: &DirectWriteEntry, string_typed_names: &FxHashSet<String>) -> bool {
    match &e.value {
        Value::String(_) => true,
        Value::Object(obj) => {
            if obj.contains_key("$concat")
                || obj.contains_key("$call")
                || obj.contains_key("$field")
            {
                // sanitizeCss-wrapped → string at runtime
                return true;
            }
            obj.get("$signal")
                .and_then(|v| v.as_str())
                .map(|name| string_typed_names.contains(name))
                .unwrap_or(false)
        }
        _ => false,
    }
}

/// Wrap a direct-write style value: `` `${value}px` `` on dimension props
/// for non-string values, bare otherwise. Shared by the D1 apply channel
/// and the B2 effect channel (both write the same DOM property — the px
/// decision must match, same doctrine as `entry_needs_sanitize`).
fn style_write_value(
    e: &DirectWriteEntry,
    value_js: &str,
    string_typed_names: &FxHashSet<String>,
) -> String {
    if DIMENSION_PROPS.contains(&e.prop.as_str()) && !entry_value_is_string(e, string_typed_names) {
        format!("`${{{value_js}}}px`")
    } else {
        value_js.to_string()
    }
}

// ═══════════════════════════════════════════════════════════
// Apply-line construction (Phase 1, Task 3)
// ═══════════════════════════════════════════════════════════

/// Whether a style entry value needs the `sanitizeCss` wrapper — a value
/// built from user-controllable strings ($concat) or arbitrary evaluation
/// ($call/$field) is wrapped in `sanitizeCss(...)`, mirroring the OmniView
/// `__omni_sanitizeCss` behavior (module-level helper emitted by
/// react/design.rs; the design doc's apply example §4.3.2 shows the wrapped
/// shape). Pure signal/computed references ($signal/$ident) are
/// compiler-typed numbers/strings and skip the wrapper — SAME rule for the
/// D1 apply channel and the B2 effect channel (both write the same DOM
/// property; wrapping one and not the other would leave D1's sanitization
/// bypassable through the B2 write on the same property).
fn entry_needs_sanitize(e: &DirectWriteEntry) -> bool {
    matches!(
        &e.value,
        Value::Object(obj)
            if obj.contains_key("$concat") || obj.contains_key("$call") || obj.contains_key("$field")
    )
}

/// Build `signal → apply line` map. Each apply reads the CURRENT ref
/// values of all entry signals, so injecting after any mutation of any
/// entry signal keeps the DOM in sync (idempotent double-writes when a
/// handler mutates several signals are accepted — μs cost).
pub(super) fn build_apply_map(
    sets: &DirectWriteSets,
    ref_names: &FxHashSet<String>,
    string_typed_names: &FxHashSet<String>,
) -> FxHashMap<String, Vec<String>> {
    let mut refs = ref_names.clone();
    refs.extend(sets.d1.iter().cloned());
    refs.extend(sets.d2.iter().cloned());
    let mut map: FxHashMap<String, Vec<String>> = FxHashMap::default();
    for e in &sets.entries {
        let value_js = if entry_needs_sanitize(e) {
            format!("sanitizeCss({})", format_style_value_expr(&e.value, &refs))
        } else {
            format_style_value_expr(&e.value, &refs)
        };
        let value_js = style_write_value(e, &value_js, string_typed_names);
        for sig in &e.read_signals {
            // CSS custom property 特例：`format_style_prop_key` 对 `--*` 返回
            // 带引号的 `'--x'`，而属性赋值 `el.style.'--x'` 非法——改用
            // `style.setProperty('--x', v)`。
            // TS 语法约束（TS2779）：`?.` 不能作赋值 LHS——写点只出现在
            // 事件处理器（D1 apply）与挂载后 effect（B2）中，ref 必已附着，
            // 用 `!` 非空断言（同生成产物 `__omni_activePointerId!` 先例）。
            // `--` 分支的 setProperty 是方法调用，`?.` 合法，保持空安全。
            let line = if e.prop.starts_with("--") {
                format!(
                    "{}Ref.current?.style.setProperty('{}', {});",
                    e.ref_name, e.prop, value_js
                )
            } else {
                format!(
                    "{}Ref.current!.style.{} = {};",
                    e.ref_name,
                    format_style_prop_key(&e.prop),
                    value_js
                )
            };
            map.entry(sig.clone()).or_default().push(line);
        }
    }
    map
}

/// Whether any active entry value needs the `sanitizeCss` wrapper —
/// gates the module-level helper emission at the component head (mirrors
/// the dynamic-token trigger in react/design.rs). Iterates the FULL
/// `sets.entries` collection (D1 + B2 — B2 entries are a subset pushed
/// from it), so a B2-only component with a sanitize-worthy value still
/// gets the helper.
pub(super) fn apply_needs_sanitize(sets: &DirectWriteSets) -> bool {
    sets.entries.iter().any(entry_needs_sanitize)
}

// ═══════════════════════════════════════════════════════════
// B2 effect emission (Phase 1, Task 4)
// ═══════════════════════════════════════════════════════════

/// Emit `useEffect(() => { {ref}Ref.current!.style.{prop} = value; }, [deps])`
/// for B2 entries — entries whose value reads anything beyond D1 signals
/// (computed / store field / prop / a plain signal left on useState), or
/// reads nothing at all. Those entries cannot ride the mutation-site apply
/// channel, so the style prop removal (Task 4) must be balanced by a
/// dedicated effect; deps re-run the write exactly when a read value
/// changes.
///
/// Deps = the entry's read signals MINUS D1/D2 names: D1/D2 are
/// useRef-stored, their bare names would be undeclared (TS2304) and the ref
/// identity is stable, so they add nothing to the re-run decision. The
/// remaining names resolve through the shared semantic-binding deps logic
/// (store slices / hoisted exclusion — `state::binding_deps_from_names`),
/// so a computed dep emits its bare name (a declared useMemo const, mirror
/// of the existing computed dep emission) and a store-field read follows
/// the same slice renames as every other effect dep.
///
/// `--` custom properties use `style.setProperty` (a quoted `'--x'` after
/// `style.` is invalid JS) — same special case as `build_apply_map`.
///
/// Values needing sanitization ($concat/$call/$field) are wrapped in
/// `sanitizeCss(...)` — identical decision to `build_apply_map`
/// (`entry_needs_sanitize`): both channels write the same DOM property and
/// must encode the value identically (fix round 1).
pub(super) fn gen_ref_style_effects(
    w: &mut crate::codegen::CodeWriter,
    sets: &DirectWriteSets,
    ref_names: &FxHashSet<String>,
    store_signal_map: &FxHashMap<String, String>,
    store_field_renames: &FxHashMap<String, String>,
    hoisted_signals: &FxHashSet<String>,
    string_typed_names: &FxHashSet<String>,
) {
    if sets.b2_entries.is_empty() {
        return;
    }
    let mut refs = ref_names.clone();
    refs.extend(sets.d1.iter().cloned());
    refs.extend(sets.d2.iter().cloned());
    for e in &sets.b2_entries {
        // Fix round 1: same sanitization decision as the D1 apply channel
        // (entry_needs_sanitize) — both channels write the same DOM
        // property, so a B2 raw write would bypass the D1 sanitizer on
        // that property (same value, two encodings). Pure ident values
        // stay bare, exactly like the D1 channel. Task 15: same px rule as
        // the D1 channel (style_write_value) — numeric values on dimension
        // props get the `` `${...}px` `` template (a raw numeric write is
        // invalid unit-less CSS and TS2322 against the DOM lib).
        let value_js = if entry_needs_sanitize(e) {
            format!("sanitizeCss({})", format_style_value_expr(&e.value, &refs))
        } else {
            format_style_value_expr(&e.value, &refs)
        };
        let value_js = style_write_value(e, &value_js, string_typed_names);
        let dep_names: FxHashSet<String> = e
            .read_signals
            .iter()
            .filter(|s| !sets.d1.contains(*s) && !sets.d2.contains(*s))
            .cloned()
            .collect();
        let deps_str = super::state::binding_deps_from_names(
            &dep_names,
            store_signal_map,
            store_field_renames,
            hoisted_signals,
        );
        w.emit_fmt(format_args!("useEffect(() => {{"));
        w.indent();
        // 同 D1 apply 通道：TS2779 约束下赋值 LHS 用 `!` 非空断言
        // （effect 挂载后运行，ref 必已附着）；`--` 分支 setProperty 保持 `?.`。
        if e.prop.starts_with("--") {
            w.emit_fmt(format_args!(
                "{}Ref.current?.style.setProperty('{}', {});",
                e.ref_name, e.prop, value_js
            ));
        } else {
            w.emit_fmt(format_args!(
                "{}Ref.current!.style.{} = {};",
                e.ref_name,
                format_style_prop_key(&e.prop),
                value_js
            ));
        }
        w.dedent();
        w.emit_fmt(format_args!("}}, [{}]);", deps_str));
        w.emit("");
    }
}
