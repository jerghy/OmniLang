//! Direct-write channel classification (ADR-071 decision 2, Phase 1).
//!
//! Classifies component signals consumed by dynamic `style: { expr }`
//! bindings on `@ref`'d elements (the "property always equals expr"
//! semantic) into storage domains:
//!
//! * D1 (direct-write): signal's only output reads are inside active
//!   entries → stored in `useRef`, mutations inject a direct DOM write.
//! * D2 (write-only): signal has no output reads (only mutation-RHS
//!   self-reads) → stored in `useRef`, no apply needed.
//! * B2 (shared): signal has render-path reads (text/class/prop/...)
//!   → stays `useState` + `useEffect([deps])` direct write.
//!
//! Computed signals and store fields NEVER enter D1/D2 (they must keep
//! their render-path derivation); entries reading them are B2.
//!
//! Read-domain boundary (fix round F2): output-read classification counts
//! only (a) view-tree reads, (b) inline event-handler body reads, and
//! (c) active-entry value reads (subtracted for D1). Reads inside
//! called-fn/effect bodies — including mutation-RHS self-reads — never
//! participate in D1/D2 determination: D1/D2 names resolve to
//! `{name}Ref.current` there (always the live value; Task 3 emission
//! contract), so the render path never consumes them and the semantics
//! are equivalent.

use crate::ir::{ComponentIR, ViewTree};
use rustc_hash::FxHashSet;
use serde_json::Value;

/// One activated style entry: `{ref}Ref.current.style.{prop} = value`.
#[derive(Debug, Clone)]
pub(crate) struct DirectWriteEntry {
    pub ref_name: String,
    pub prop: String,
    pub value: Value,
    pub read_signals: FxHashSet<String>,
}

/// Classification result consumed by the emission passes.
#[derive(Debug, Default)]
pub(crate) struct DirectWriteSets {
    /// D1 signals: useRef storage + mutation-site apply injection.
    pub d1: FxHashSet<String>,
    /// D2 signals: useRef storage, no apply.
    pub d2: FxHashSet<String>,
    /// Activated (D1) entries — order follows view traversal.
    pub entries: Vec<DirectWriteEntry>,
    /// B2 entries: useEffect([deps]) direct writes.
    pub b2_entries: Vec<DirectWriteEntry>,
}

/// True when a style value is dynamic (contains $signal/$ident/$field/
/// $call/$concat meta-objects) — mirrors the shared/style.rs dispatch.
fn is_dynamic_style_value(val: &Value) -> bool {
    match val {
        Value::Object(obj) => {
            obj.contains_key("$signal")
                || obj.contains_key("$ident")
                || obj.contains_key("$field")
                || obj.contains_key("$call")
                || obj.contains_key("$concat")
        }
        _ => false,
    }
}

/// Collect every identifier name referenced by a style value JSON tree
/// ($signal / $ident leaf nodes).
fn collect_value_reads(val: &Value, acc: &mut FxHashSet<String>) {
    match val {
        Value::Object(obj) => {
            if let Some(sig) = obj.get("$signal").and_then(|v| v.as_str()) {
                acc.insert(sig.to_string());
                return;
            }
            if let Some(ident) = obj.get("$ident").and_then(|v| v.as_str()) {
                acc.insert(ident.to_string());
                return;
            }
            for v in obj.values() {
                collect_value_reads(v, acc);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect_value_reads(v, acc);
            }
        }
        _ => {}
    }
}

/// Walk the view tree collecting elements (ref binding + dynamic style
/// entries). `in_for` guards activation condition 3 (@for + ref inactive).
fn collect_active_entries(node: &ViewTree, in_for: bool, out: &mut Vec<DirectWriteEntry>) {
    match node {
        ViewTree::Element(el) => {
            // ElementIR has no for_binding field — elements inside @for are
            // reached via the ListRendering arm below with in_for=true.
            let nested_for = in_for;
            if let Some(ref_name) = &el.ref_binding {
                if let Some(obj) = el.inline_style.as_ref().and_then(|v| v.as_object()) {
                    for (prop, val) in obj {
                        if !nested_for && is_dynamic_style_value(val) {
                            let mut reads = FxHashSet::default();
                            collect_value_reads(val, &mut reads);
                            out.push(DirectWriteEntry {
                                ref_name: ref_name.clone(),
                                prop: prop.clone(),
                                value: val.clone(),
                                read_signals: reads,
                            });
                        }
                    }
                }
            }
            for child in &el.children {
                collect_active_entries(child, nested_for, out);
            }
        }
        ViewTree::Conditional(cond) => {
            for b in &cond.branches {
                if let Some(body) = &b.body {
                    collect_active_entries(body, in_for, out);
                }
            }
            if let Some(fallback) = &cond.fallback {
                collect_active_entries(fallback, in_for, out);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(body) = &list.body {
                collect_active_entries(body, true, out);
            }
            if let Some(empty_body) = &list.empty_body {
                collect_active_entries(empty_body, true, out);
            }
        }
        _ => {}
    }
}

/// Output reads of a signal OUTSIDE active entry values — the set of
/// identifiers referenced by render output (text interpolation, class
/// bindings, props, non-active style entries) plus inline event-handler
/// body reads. Reads inside called-fn/effect bodies (incl. mutation-RHS
/// self-reads) are intentionally NOT collected — read-domain boundary,
/// see module doc (fix round F2).
///
/// Reuses the existing collectors (collect_view_read_signals +
/// collect_view_inline_handler_reads); the active-entry reads are
/// subtracted afterwards.
fn collect_output_reads(
    comp: &ComponentIR,
    active_entries: &[DirectWriteEntry],
) -> FxHashSet<String> {
    use super::state::{collect_view_inline_handler_reads, collect_view_read_signals};
    let mut reads = FxHashSet::default();
    if let Some(view) = &comp.view {
        reads.extend(collect_view_read_signals(view));
        collect_view_inline_handler_reads(view, &mut reads);
    }
    // Read-domain boundary (fix round F2): no fn/effect body reads are
    // collected here — classification counts view reads + inline-handler
    // reads + active-entry value reads only. fn/effect body reads (incl.
    // mutation-RHS self-reads) never participate in D1/D2 determination:
    // D1/D2 names resolve to `{name}Ref.current` inside fn/effect bodies
    // (always the live value — Task 3 emission contract), so the render
    // path never consumes them and semantics are equivalent.
    for entry in active_entries {
        for s in &entry.read_signals {
            reads.remove(s);
        }
    }
    reads
}

/// Classify component signals into D1 / D2; everything else stays on the
/// existing useState path. Only `comp.signals` (not computeds) qualify.
pub(crate) fn classify_direct_write(comp: &ComponentIR) -> DirectWriteSets {
    let mut sets = DirectWriteSets::default();
    if let Some(view) = &comp.view {
        collect_active_entries(view, false, &mut sets.entries);
    }
    if sets.entries.is_empty() {
        return sets;
    }

    // Signal names appearing in any active entry value.
    let mut entry_consumed: FxHashSet<String> = FxHashSet::default();
    for e in &sets.entries {
        entry_consumed.extend(e.read_signals.iter().cloned());
    }

    // Output reads outside active entries (view + handlers + fns).
    let output_reads = collect_output_reads(comp, &sets.entries);

    let signal_names: FxHashSet<String> = comp.signals.iter().map(|s| s.name.clone()).collect();

    for sig in &comp.signals {
        if !signal_names.contains(&sig.name) {
            continue;
        }
        let is_mutated = comp_mutated_signals(comp).contains(&sig.name);
        if !is_mutated {
            continue;
        }
        if entry_consumed.contains(&sig.name) && !output_reads.contains(&sig.name) {
            sets.d1.insert(sig.name.clone());
        } else if !entry_consumed.contains(&sig.name) && !output_reads.contains(&sig.name) {
            sets.d2.insert(sig.name.clone());
        }
    }

    // B2 entries: any entry that fails to read ONLY D1 signals. Empty
    // read_signals (pure-literal $concat / zero-arg $call) is NOT "all D1":
    // such an entry consumes no D1 signal, so Task 3's apply would never
    // cover it and the style prop would silently never take effect — it
    // must stay on the useEffect path (fix round F1). A non-D1 read
    // (computed, store field, prop, or a plain signal left on the useState
    // path) also keeps the entry on the useEffect([deps]) direct-write
    // path (module doc + test contract: computed-driven entries are B2). A
    // read can never be D2 here (D2 requires absence from every entry's
    // read set).
    for e in sets.entries.iter() {
        let all_d1 =
            !e.read_signals.is_empty() && e.read_signals.iter().all(|s| sets.d1.contains(s));
        if !all_d1 {
            sets.b2_entries.push(e.clone());
        }
    }
    sets
}

/// Mutated component signal names (delegates to the existing collector).
fn comp_mutated_signals(comp: &ComponentIR) -> FxHashSet<String> {
    // collect_signal_mutations lives in state.rs and needs called_fns —
    // reuse the same wiring as mod.rs does for mutated_signals.
    let called = super::function::collect_called_local_fns(comp);
    super::state::collect_signal_mutations(comp, &called)
}

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
