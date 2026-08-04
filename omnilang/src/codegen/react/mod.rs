//! React codegen: IR -> React TSX components + CSS modules + store hooks.
//!
//! Each component generates a `.tsx` React functional component with hooks
//! and a `.module.css` CSS module file. Each store generates a `useXxx.ts`
//! custom hook file.

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::build_component_map;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::{format_json_value, JSON_REACT};
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

pub(super) mod component;
pub(super) mod design;
pub(super) mod expr;
pub(super) mod motion;
#[cfg(test)]
pub(super) mod tests;
pub(super) mod types;
pub(super) mod utils;
pub(crate) mod view;

/// React codegen — stores are always generated as Zustand v5 stores.
#[derive(Debug, Default)]
pub struct ReactCodegen;

impl Codegen for ReactCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &FxHashMap<String, String>) -> CompileOutput {
        // The component/store loop is hand-rolled (instead of the shared
        // generate_per_component) because store generation needs the whole
        // IRProgram: user struct/union types (deferral #49) are declared
        // INLINE in each generated file that references them, and the
        // referenced-type scan must know every user-declared name. Inline
        // stores (`store Name { ... }` in a component body) lower to
        // module-level StoreIRs (Phase 5.2, deferral #37) and emit their
        // useXxx.ts Zustand store files exactly like module-level stores.
        let mut output = CompileOutput::success(Vec::new());
        for (name, comp) in ir.all_components() {
            output.files.extend(self.gen_component_file(name, comp, ir));
        }
        for (name, store) in ir.all_stores() {
            output
                .files
                .extend(self.gen_store_file_typed(name, store, ir));
        }
        // Module-level design blocks → design.css + design runtime files
        // (useDesignTokens.ts / useThemeStore.ts) in a dedicated `__design/`
        // subdirectory of the source file's output directory (Task 4.4
        // naming isolation — the runtime basenames can never collide with a
        // user store's hook file). Designs are grouped per source file
        // (DesignIR.source_file), so multiple design blocks in one .omni
        // file share a single set of outputs.
        output.files.extend(design::gen_design_files(ir));
        output
    }

    fn gen_component_file(
        &self,
        name: &str,
        comp: &ComponentIR,
        ir: &IRProgram,
    ) -> Vec<CompiledFile> {
        let component_map = build_component_map(ir);
        let dir_prefix = source_dir_prefix(comp.source_file.as_deref());
        vec![
            CompiledFile {
                path: format!("{}{}.tsx", dir_prefix, name),
                content: self.gen_component_tsx(name, comp, ir, &component_map),
            },
            CompiledFile {
                path: format!("{}{}.module.css", dir_prefix, name),
                content: self.gen_component_css(name, comp),
            },
        ]
    }
}

/// Extract the output directory prefix from a source file path.
///
/// Example: `"src/components/Counter.omni"` → `"components/"`
/// Example: `"src/stores/TodoStore.omni"` → `"stores/"`
/// Example: `"src/main.omni"` → `""` (root-level files go to output root)
fn source_dir_prefix(source_file: Option<&str>) -> String {
    let path = match source_file {
        Some(p) => p,
        None => return String::new(),
    };
    // Get the parent directory of the source file
    let parent = match std::path::Path::new(path).parent() {
        Some(p) => p,
        None => return String::new(),
    };
    // Strip common source root prefixes
    let parent_str = parent.to_string_lossy();
    // Normalize backslashes to forward slashes
    let normalized = parent_str.replace('\\', "/");
    // Strip "src/" or "src" prefix if present
    let stripped = normalized
        .strip_prefix("src/")
        .or_else(|| normalized.strip_prefix("src"))
        .unwrap_or(&normalized);
    // Strip leading/trailing slashes and "."
    let trimmed = stripped.trim_matches('/').trim_matches('.');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{}/", trimmed)
    }
}

/// Compute the output directory of a source file (without the filename).
fn output_dir(source_file: Option<&str>) -> String {
    let path = match source_file {
        Some(p) => p,
        None => return String::new(),
    };
    let parent = match std::path::Path::new(path).parent() {
        Some(p) => p.to_string_lossy().replace('\\', "/"),
        None => return String::new(),
    };
    // Strip "src/" or "src" prefix
    let stripped = parent
        .strip_prefix("src/")
        .or_else(|| parent.strip_prefix("src"))
        .unwrap_or(&parent);
    let trimmed = stripped.trim_matches('/').trim_matches('.');
    if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// Compute a relative import path from one source file to another component's output.
///
/// Example:
/// - from `src/components/Header.omni` to component `Counter` at `src/components/Counter.omni`
///   → `"./Counter"`
/// - from `src/components/Header.omni` to store `TodoStore` at `src/stores/TodoStore.omni`
///   → `"../stores/useTodoStore"`
/// - from `src/main.omni` to component `Header` at `src/components/Header.omni`
///   → `"./components/Header"`
pub(super) fn relative_import_path(
    from_source: Option<&str>,
    to_source: Option<&str>,
    to_module_name: &str,
) -> String {
    let from_dir = output_dir(from_source);
    let to_dir = output_dir(to_source);

    if from_dir == to_dir {
        // Same directory — simple relative import
        return format!("./{}", to_module_name);
    }

    // Compute relative path from from_dir to to_dir
    let from_parts: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to_dir.split('/').filter(|s| !s.is_empty()).collect();

    // Find common prefix
    let mut common_len = 0;
    for (a, b) in from_parts.iter().zip(to_parts.iter()) {
        if a == b {
            common_len += 1;
        } else {
            break;
        }
    }

    let up_count = from_parts.len() - common_len;
    let down_path: Vec<&str> = to_parts[common_len..].to_vec();

    let mut result = String::new();
    if up_count == 0 {
        result.push_str("./");
    } else {
        for _ in 0..up_count {
            result.push_str("../");
        }
    }
    for part in &down_path {
        result.push_str(part);
        result.push('/');
    }
    result.push_str(to_module_name);
    result
}

impl ReactCodegen {
    /// Store generation with program access (deferral #49): user type
    /// declarations are emitted inline into the store file. The trait's
    /// `gen_store_file` override is intentionally absent — `generate` calls
    /// this method directly with the IR (the trait default would return no
    /// files, but it is never invoked for React).
    fn gen_store_file_typed(
        &self,
        name: &str,
        store: &StoreIR,
        ir: &IRProgram,
    ) -> Vec<CompiledFile> {
        let dir_prefix = source_dir_prefix(store.source_file.as_deref());
        let content = self.gen_zustand_store_typed(name, store, ir);
        vec![CompiledFile {
            path: format!("{}use{}.ts", dir_prefix, name),
            content,
        }]
    }

    /// Generate a Zustand v5 store (useStoreName.ts) — ir-less entry used
    /// by unit tests that hand-construct a StoreIR (no user-type emission).
    fn gen_zustand_store(&self, name: &str, store: &StoreIR) -> String {
        self.gen_zustand_store_inner(name, store, None)
    }

    /// Generate a Zustand v5 store (useStoreName.ts) with program access.
    ///
    /// Emits `create<T>()(...)` with signals, getter accessors and actions.
    /// Subscription happens at the consumer: components and the
    /// design-tokens hook use inline `useStore((s) => s.x)` slice selectors
    /// (store_slices.rs / design.rs, Task 2.7) — the standalone per-field
    /// selector hooks were dead code (nothing imported them) and are no
    /// longer emitted (perf M2).
    ///
    /// User struct/union/alias types referenced by the store's annotations
    /// (signals, getter types, action params/returns) are declared inline
    /// after the import line (deferral #49) — the state interface references
    /// them (`items: Todo[]`) and tsc must find a declaration.
    fn gen_zustand_store_typed(&self, name: &str, store: &StoreIR, ir: &IRProgram) -> String {
        self.gen_zustand_store_inner(name, store, Some(ir))
    }

    fn gen_zustand_store_inner(
        &self,
        name: &str,
        store: &StoreIR,
        ir: Option<&IRProgram>,
    ) -> String {
        let mut w = CodeWriter::new();
        w.emit_fmt(format_args!(
            "// Generated by OmniLang — Zustand store: use{}",
            name
        ));
        w.emit("import { create } from 'zustand';");
        // ── User type declarations (deferral #49) ──
        if let Some(ir) = ir {
            let user_types = types::user_type_names(ir);
            let mut type_refs: FxHashSet<String> = FxHashSet::default();
            for ty in store.signals.values() {
                types::collect_annotation_refs(ty, &user_types, &mut type_refs);
            }
            for ty in store.getter_types.values() {
                types::collect_annotation_refs(ty, &user_types, &mut type_refs);
            }
            // Action RETURN types are not scanned: the emitted interface
            // types every action `=> void` — a collected return type would
            // be an unused declaration (eslint no-unused-vars).
            for action in &store.actions {
                for p in &action.params {
                    if let Some(t) = &p.r#type {
                        types::collect_annotation_refs(t, &user_types, &mut type_refs);
                    }
                }
            }
            types::close_type_refs(ir, &user_types, &mut type_refs);
            types::emit_type_declarations(&mut w, ir, &type_refs);
            w.emit("");
        }

        // State interface
        let state_interface = format!("{}State", name);
        w.emit_fmt(format_args!("interface {} {{", state_interface));

        for (sig_name, sig_type) in &store.signals {
            let ts_type = JsTypeFormatter.format_type(sig_type.kind.as_str(), &sig_type.type_args);
            w.emit_fmt(format_args!("  {}: {};", sig_name, ts_type));
        }
        for getter_name in store.getters.keys() {
            let ts_type = store
                .getter_types
                .get(getter_name)
                .map(|ty| JsTypeFormatter.format_type(ty.kind.as_str(), &ty.type_args))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "any".into());
            w.emit_fmt(format_args!("  readonly {}: {};", getter_name, ts_type));
        }
        for action in &store.actions {
            let params: Vec<String> = action
                .params
                .iter()
                .map(|p| {
                    let name = escape_js_ident(&p.name);
                    match &p.r#type {
                        Some(ty) => {
                            let ts_type =
                                JsTypeFormatter.format_type(ty.kind.as_str(), &ty.type_args);
                            if ts_type.is_empty() {
                                format!("{}: any", name)
                            } else {
                                format!("{}: {}", name, ts_type)
                            }
                        }
                        None => format!("{}: any", name),
                    }
                })
                .collect();
            if params.is_empty() {
                w.emit_fmt(format_args!("  {}: () => void;", action.name));
            } else {
                w.emit_fmt(format_args!(
                    "  {}: ({}) => void;",
                    action.name,
                    params.join(", ")
                ));
            }
        }
        w.emit("}");
        w.emit("");

        // Store names (signals + getters) — identifiers inside action bodies
        // and getter accessors are renamed to `state.<name>` (C8, HIGH #1).
        let store_signal_names: FxHashSet<String> = store.signals.keys().cloned().collect();
        let store_getter_names: FxHashSet<String> = store.getters.keys().cloned().collect();
        let store_names: FxHashSet<String> = store_signal_names
            .iter()
            .chain(store_getter_names.iter())
            .cloned()
            .collect();
        let action_renames: FxHashMap<String, String> = store_names
            .iter()
            .map(|n| (n.clone(), format!("state.{}", n)))
            .collect();

        // Check if any action body needs get()
        let needs_get = store
            .actions
            .iter()
            .any(|a| action_needs_zustand_state(a, &store_signal_names, &store_getter_names));
        let create_params = if needs_get { "(set, get)" } else { "(set)" };

        let has_getters = !store.getters.is_empty();
        let set_fn = if has_getters { "__set" } else { "set" };

        if has_getters {
            // ── Getter freeze fix (Phase 5.2): getters are emitted as
            // COMPUTED VALUES inside a `__recompute` helper, evaluated in
            // dependency order (a getter referencing another getter reads
            // the freshly computed const). The old accessor-property
            // emission froze into stale data props on the first set() —
            // Zustand's Object.assign merge invokes the accessor once and
            // copies its value, so `s.totalCount` never updated again
            // (BUG-C follow-up, verified in-browser on TodoStore).
            //
            // `__set` wraps every store write: it merges the mutation
            // partial into the current state and re-runs `__recompute`, so
            // getter values are always fresh data properties. The initial
            // state goes through the same helper (`__initial`), so getters
            // are materialized from the very first render.
            w.emit_fmt(format_args!(
                "export const use{} = create<{}>()({} => {{",
                name, state_interface, create_params
            ));
            w.indent();
            emit_getter_helper(
                &mut w,
                name,
                store,
                &store_signal_names,
                &store_getter_names,
            );
            // Initial state — signals only; getters materialized by __getters.
            w.emit("const __initial = __getters({");
            w.indent();
            for sig_name in store.signals.keys() {
                let init = store
                    .signal_defaults
                    .get(sig_name)
                    .map(|v| format_json_value(v, &JSON_REACT))
                    .unwrap_or_else(|| "null".into());
                w.emit_fmt(format_args!("{}: {},", sig_name, init));
            }
            w.dedent();
            w.emit("});");
            // Set wrapper — recompute getters after every mutation merge.
            // Only stores with actions need it (read-only getter stores emit
            // no mutations; emitting an unused local would trip the eslint
            // no-unused-vars gate). `_replace` stays for signature parity
            // with Zustand's setState; our emissions never pass replace.
            if !store.actions.is_empty() {
                w.emit("const __set: typeof set = (partial, _replace) => {");
                w.indent();
                w.emit("set((state) => {");
                w.indent();
                w.emit("const next = typeof partial === 'function' ? partial(state) : partial;");
                w.emit("return __getters({ ...state, ...next });");
                w.dedent();
                w.emit("});");
                w.dedent();
                w.emit("};");
            }
            w.emit("return {");
            w.indent();
            w.emit("...__initial,");
            emit_store_actions(
                &mut w,
                store,
                &action_renames,
                &store_signal_names,
                &store_getter_names,
                set_fn,
            );
            w.dedent();
            w.emit("};");
            w.dedent();
            w.emit("});");
        } else {
            // create() call — v5 curried form (no getters: plain object).
            w.emit_fmt(format_args!(
                "export const use{} = create<{}>()({} => ({{",
                name, state_interface, create_params
            ));
            w.indent();

            // Signal defaults
            for sig_name in store.signals.keys() {
                let init = store
                    .signal_defaults
                    .get(sig_name)
                    .map(|v| format_json_value(v, &JSON_REACT))
                    .unwrap_or_else(|| "null".into());
                w.emit_fmt(format_args!("{}: {},", sig_name, init));
            }

            emit_store_actions(
                &mut w,
                store,
                &action_renames,
                &store_signal_names,
                &store_getter_names,
                set_fn,
            );

            w.dedent();
            w.emit("}));");
        }

        w.into_string()
    }
}

/// Emit the getter-materialization machinery (Phase 5.2 getter freeze fix):
///
/// ```ts
/// type <Name>Signals = Pick<<Name>State, "<signal>" | ...>;
///
/// const __getters = (s: <Name>Signals) => {
///   const <getter> = <body with s.<signal> and bare getter refs>;
///   ...
///   return { ...s, <getter>, ... };
/// };
/// ```
///
/// Getters are COMPUTED VALUES, never accessor properties: Zustand's
/// Object.assign merge flattens accessor getters into stale data props on
/// the first set() (the freeze bug — TodoStore.totalCount never updated),
/// so every mutation path re-runs `__getters` over the merged state and
/// the initial state goes through it too. The signals-only param type gives
/// the initial literal full contextual typing (`[]` → `number[]`, never
/// `never[]`) while getter bodies can safely read `s.<signal>`. `Pick`
/// (not a redeclared member type) avoids duplicating struct-type references
/// like `Todo[]` — the store interface's own instance stays the only
/// TS2304-class occurrence.
///
/// Getter bodies rename store signals to `s.<signal>` and already-computed
/// getters to their bare const name — never to `s.<getter>` (a stale data
/// property). Getter→getter cycles (semantically invalid but defensively
/// handled) fall back to `s.<getter>` reads for the cyclic members.
fn emit_getter_helper(
    w: &mut CodeWriter,
    name: &str,
    store: &StoreIR,
    store_signal_names: &FxHashSet<String>,
    store_getter_names: &FxHashSet<String>,
) {
    // Signals-only view of the state interface — the initial literal is
    // checked against this, so every member is present and array defaults
    // get contextual typing. Sorted keys keep the emitted type stable.
    let mut signal_keys: Vec<&String> = store.signals.keys().collect();
    signal_keys.sort();
    let pick = format!(
        "type {}Signals = Pick<{}State, {}>;",
        name,
        name,
        signal_keys
            .iter()
            .map(|k| format!("\"{}\"", k))
            .collect::<Vec<_>>()
            .join(" | ")
    );
    w.emit(&pick);
    w.emit("");

    let order = topo_order_getters(store, store_signal_names, store_getter_names);
    w.emit_fmt(format_args!("const __getters = (s: {}Signals) => {{", name));
    w.indent();
    let mut renamed: FxHashSet<String> = FxHashSet::default();
    for getter_name in &order {
        let getter_expr = store
            .getters
            .get(getter_name.as_str())
            .expect("topo order must contain declared getters only");
        // Signals → `s.x`; getters computed so far → bare const; getters not
        // yet computed (cycle leftovers) → `s.x` (stale but compiles).
        let mut renames: FxHashMap<String, String> = store_signal_names
            .iter()
            .map(|n| (n.clone(), format!("s.{}", n)))
            .collect();
        for g in store_getter_names.iter() {
            if renamed.contains(g.as_str()) {
                renames.insert(g.clone(), g.clone());
            } else {
                renames.insert(g.clone(), format!("s.{}", g));
            }
        }
        let body = expr::format_expr_ir_tsx_renamed(getter_expr, &FxHashSet::default(), &renames);
        w.emit_fmt(format_args!("const {} = {};", getter_name, body));
        renamed.insert(getter_name.clone());
    }
    w.emit_fmt(format_args!("return {{ ...s, {} }};", order.join(", ")));
    w.dedent();
    w.emit("};");
}

/// Topologically order the getters by their getter-to-getter dependencies
/// (a getter reading another getter must be evaluated after it). Members of
/// a dependency cycle are appended in declaration order — the recompute
/// emission falls back to `s.<getter>` reads for them.
fn topo_order_getters(
    store: &StoreIR,
    _store_signal_names: &FxHashSet<String>,
    store_getter_names: &FxHashSet<String>,
) -> Vec<String> {
    let mut deps_of: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let mut dependents: FxHashMap<String, Vec<String>> = FxHashMap::default();
    let mut in_degree: FxHashMap<String, usize> = FxHashMap::default();
    for (g, body) in &store.getters {
        let mut ids = FxHashSet::default();
        component::collect_expr_identifiers(body, &mut ids);
        let deps: FxHashSet<String> = ids
            .iter()
            .filter(|n| store_getter_names.contains(n.as_str()) && n.as_str() != g)
            .cloned()
            .collect();
        in_degree.insert(g.clone(), deps.len());
        for d in &deps {
            dependents.entry(d.clone()).or_default().push(g.clone());
        }
        deps_of.insert(g.clone(), deps);
    }
    // Kahn's algorithm over getter→getter edges.
    let mut ready: Vec<String> = store
        .getters
        .keys()
        .filter(|g| in_degree.get(g.as_str()) == Some(&0))
        .cloned()
        .collect();
    // Deterministic order — declare the ready set in declaration order.
    ready.sort_by(|a, b| {
        let ia = store.getters.keys().position(|k| k == a).unwrap_or(0);
        let ib = store.getters.keys().position(|k| k == b).unwrap_or(0);
        ia.cmp(&ib)
    });
    let mut order: Vec<String> = Vec::new();
    while let Some(g) = ready.pop() {
        order.push(g.clone());
        if let Some(ds) = dependents.get(g.as_str()) {
            for d in ds {
                let deg = in_degree
                    .get_mut(d.as_str())
                    .expect("dependent getter must be registered");
                *deg -= 1;
                if *deg == 0 {
                    ready.push(d.clone());
                }
            }
        }
    }
    // Cycle leftovers — declaration order (the emit falls back to `s.` reads).
    for g in store.getters.keys() {
        if !order.contains(g) {
            order.push(g.clone());
        }
    }
    order
}

/// Emit the store action members of the creator object.
fn emit_store_actions(
    w: &mut CodeWriter,
    store: &StoreIR,
    action_renames: &FxHashMap<String, String>,
    store_signal_names: &FxHashSet<String>,
    store_getter_names: &FxHashSet<String>,
    set_fn: &str,
) {
    for action in &store.actions {
        let params: Vec<String> = action
            .params
            .iter()
            .map(|p| escape_js_ident(&p.name))
            .collect();
        let params_str = params.join(", ");
        w.emit_fmt(format_args!("{}: ({}) => {{", action.name, params_str));
        w.indent();

        // Emit const state = get(); when the action body references store
        // signals or getters (C8)
        emit_zustand_action_prologue(w, action, store_signal_names, store_getter_names);

        gen_zustand_action_stmts(w, &action.body.statements, action_renames, set_fn);
        w.dedent();
        w.emit("},");
    }
}

/// Generate a statement inside a Zustand store action body.
/// Mutations to store signals are wrapped in `set((state) => (...))`.
/// Identifiers matching store signals/getters are renamed to `state.X` at
/// the AST level (C8) — the action prologue emits `const state = get();`
/// when needed. String literals and object keys are never touched.
fn gen_zustand_action_stmt(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    renames: &FxHashMap<String, String>,
    set_fn: &str,
) {
    /// Format an expression with the store rename map applied.
    fn fmt_expr(expr: &ExprIR, renames: &FxHashMap<String, String>) -> String {
        expr::format_expr_ir_tsx_renamed(expr, &FxHashSet::default(), renames)
    }
    match stmt {
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                format_zustand_mutation(w, intent, renames, set_fn);
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = fmt_expr(expr, renames);
            w.emit_fmt(format_args!("{};", line));
        }
        StatementIR::LetStmtIR {
            name,
            value,
            r#type: _,
            is_mut,
        } => {
            let rhs = value
                .as_ref()
                .map(|v| fmt_expr(v, renames))
                .unwrap_or_else(|| "null".into());
            // Task 5.1 (deferral #36): `is_mut` decides the keyword — `let`
            // for reassigned bindings, `const` for immutable ones.
            let kw = if *is_mut { "let" } else { "const" };
            w.emit_fmt(format_args!("{kw} {} = {};", name, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = fmt_expr(condition, renames);
            w.emit_fmt(format_args!("if ({}) {{", cond));
            w.indent();
            gen_zustand_action_stmts_inner(w, &then_body.statements, renames, false, set_fn);
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif = fmt_expr(elif_cond, renames);
                w.emit_fmt(format_args!("}} else if ({}) {{", elif));
                w.indent();
                gen_zustand_action_stmts_inner(w, &elif_body.statements, renames, false, set_fn);
                w.dedent();
            }
            if let Some(ref eb) = else_body {
                w.emit("} else {");
                w.indent();
                gen_zustand_action_stmts_inner(w, &eb.statements, renames, false, set_fn);
                w.dedent();
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let v = value
                .as_ref()
                .map(|v| fmt_expr(v, renames))
                .unwrap_or_else(|| "null".into());
            w.emit_fmt(format_args!("return {};", v));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = fmt_expr(condition, renames);
            w.emit_fmt(format_args!("while ({}) {{", cond));
            w.indent();
            gen_zustand_action_stmts_inner(w, &body.statements, renames, false, set_fn);
            w.dedent();
            w.emit("}");
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = fmt_expr(iterable, renames);
            w.emit_fmt(format_args!("for (const {} of {}) {{", var_name, iter));
            w.indent();
            gen_zustand_action_stmts_inner(w, &body.statements, renames, false, set_fn);
            w.dedent();
            w.emit("}");
        }
        StatementIR::BlockStmtIR { body } => {
            w.emit("{");
            w.indent();
            gen_zustand_action_stmts_inner(w, &body.statements, renames, false, set_fn);
            w.dedent();
            w.emit("}");
        }
        StatementIR::ThrowStmtIR { expr } => {
            let e = fmt_expr(expr, renames);
            w.emit_fmt(format_args!("throw {};", e));
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var,
            catch_body,
            finally_body,
        } => {
            w.emit("try {");
            w.indent();
            gen_zustand_action_stmts_inner(w, &try_body.statements, renames, false, set_fn);
            w.dedent();
            if let Some(ref cv) = catch_var {
                w.emit_fmt(format_args!("}} catch ({}) {{", cv));
            } else if catch_body.is_some() {
                w.emit("} catch (_) {");
            }
            if let Some(ref cb) = catch_body {
                w.indent();
                gen_zustand_action_stmts_inner(w, &cb.statements, renames, false, set_fn);
                w.dedent();
            }
            if let Some(ref fb) = finally_body {
                w.emit("} finally {");
                w.indent();
                gen_zustand_action_stmts_inner(w, &fb.statements, renames, false, set_fn);
                w.dedent();
            }
            w.emit("}");
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                "()".to_string()
            } else {
                let formatted: Vec<String> = args.iter().map(|a| fmt_expr(a, renames)).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit_fmt(format_args!("{}{};", event_name, arg_str));
        }
        StatementIR::BreakStmtIR => w.emit("break;"),
        StatementIR::ContinueStmtIR => w.emit("continue;"),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Emit a `const state = get();` line for Zustand action body prologue.
/// Emitted when the action reads or mutates any store signal or getter (C8).
fn emit_zustand_action_prologue(
    w: &mut CodeWriter,
    action: &FunctionIR,
    signal_names: &FxHashSet<String>,
    getter_names: &FxHashSet<String>,
) {
    if action_needs_zustand_state(action, signal_names, getter_names) {
        w.emit("const state = get();");
    }
}

/// Returns true if the action body needs `state.` access — i.e. any store
/// signal or getter identifier is read in an expression (these are renamed
/// to `state.X`, CRITICAL/HIGH #1) or a store signal is mutated via a
/// compound mutation. A DirectAssign emits `set({ key: val })` and only
/// needs state when its value expression reads a store name (covered by the
/// read walker, which traverses ALL ExprIR variants including list/object
/// literals).
fn action_needs_zustand_state(
    action: &FunctionIR,
    signal_names: &FxHashSet<String>,
    getter_names: &FxHashSet<String>,
) -> bool {
    let mut store_names: FxHashSet<String> = signal_names.clone();
    store_names.extend(getter_names.iter().cloned());
    if store_names.is_empty() {
        return false;
    }
    action
        .body
        .statements
        .iter()
        .any(|s| stmt_references_store_name(s, &store_names))
        || block_needs_zustand_state(&action.body, signal_names)
}

/// Returns true if any statement in the block mutates a store signal through
/// a compound mutation (Increment/Decrement/Multiply/Divide/Modulo/Push/Pop).
fn block_needs_zustand_state(block: &BlockIR, signal_names: &FxHashSet<String>) -> bool {
    block
        .statements
        .iter()
        .any(|s| stmt_needs_zustand_state(s, signal_names))
}

fn stmt_needs_zustand_state(stmt: &StatementIR, signal_names: &FxHashSet<String>) -> bool {
    match stmt {
        StatementIR::MutationStmt {
            intent: Some(intent),
        } => match intent {
            MutationIntent::Increment { target, .. }
            | MutationIntent::Decrement { target, .. }
            | MutationIntent::Multiply { target, .. }
            | MutationIntent::Divide { target, .. }
            | MutationIntent::Modulo { target, .. }
            | MutationIntent::Push { target, .. }
            | MutationIntent::Pop { target } => signal_names.contains(&target.name),
            // DirectAssign uses set({}) and does not need state
            MutationIntent::DirectAssign { .. } => false,
            #[allow(unreachable_patterns)]
            _ => {
                unreachable!("Unknown MutationIntent variant — update codegen for new variant")
            }
        },
        StatementIR::IfStmtIR {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            block_needs_zustand_state(then_body, signal_names)
                || else_ifs
                    .iter()
                    .any(|(_, b)| block_needs_zustand_state(b, signal_names))
                || else_body
                    .as_ref()
                    .is_some_and(|b| block_needs_zustand_state(b, signal_names))
        }
        StatementIR::WhileStmtIR { body, .. } => block_needs_zustand_state(body, signal_names),
        StatementIR::ForInStmtIR { body, .. } => block_needs_zustand_state(body, signal_names),
        StatementIR::BlockStmtIR { body } => block_needs_zustand_state(body, signal_names),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            block_needs_zustand_state(try_body, signal_names)
                || catch_body
                    .as_ref()
                    .is_some_and(|b| block_needs_zustand_state(b, signal_names))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| block_needs_zustand_state(b, signal_names))
        }
        _ => false,
    }
}

/// Complete statement walker — returns true if any identifier in the
/// statement (including inside list/object literals) matches a store name.
fn stmt_references_store_name(stmt: &StatementIR, store_names: &FxHashSet<String>) -> bool {
    match stmt {
        StatementIR::ExprStmtIR { expr } => expr_references_store_name(expr, store_names),
        StatementIR::LetStmtIR { value, .. } => value
            .as_ref()
            .is_some_and(|v| expr_references_store_name(v, store_names)),
        StatementIR::ReturnStmtIR { value } => value
            .as_ref()
            .is_some_and(|v| expr_references_store_name(v, store_names)),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            expr_references_store_name(condition, store_names)
                || block_references_store_name(then_body, store_names)
                || else_ifs.iter().any(|(c, b)| {
                    expr_references_store_name(c, store_names)
                        || block_references_store_name(b, store_names)
                })
                || else_body
                    .as_ref()
                    .is_some_and(|b| block_references_store_name(b, store_names))
        }
        StatementIR::WhileStmtIR { condition, body } => {
            expr_references_store_name(condition, store_names)
                || block_references_store_name(body, store_names)
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            expr_references_store_name(iterable, store_names)
                || block_references_store_name(body, store_names)
        }
        StatementIR::BlockStmtIR { body } => block_references_store_name(body, store_names),
        StatementIR::ThrowStmtIR { expr } => expr_references_store_name(expr, store_names),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            block_references_store_name(try_body, store_names)
                || catch_body
                    .as_ref()
                    .is_some_and(|b| block_references_store_name(b, store_names))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| block_references_store_name(b, store_names))
        }
        StatementIR::MutationStmt {
            intent: Some(intent),
        } => match intent {
            MutationIntent::DirectAssign { value, .. } => value
                .as_ref()
                .is_some_and(|v| expr_references_store_name(v, store_names)),
            MutationIntent::Increment { amount, .. }
            | MutationIntent::Decrement { amount, .. }
            | MutationIntent::Multiply { amount, .. }
            | MutationIntent::Divide { amount, .. }
            | MutationIntent::Modulo { amount, .. } => amount
                .as_ref()
                .is_some_and(|a| expr_references_store_name(a, store_names)),
            MutationIntent::Push { item, .. } => item
                .as_ref()
                .is_some_and(|i| expr_references_store_name(i, store_names)),
            MutationIntent::Pop { .. } => false,
            #[allow(unreachable_patterns)]
            _ => {
                unreachable!("Unknown MutationIntent variant — update codegen for new variant")
            }
        },
        StatementIR::EventEmitStmtIR { args, .. } => args
            .iter()
            .any(|a| expr_references_store_name(a, store_names)),
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

fn block_references_store_name(block: &BlockIR, store_names: &FxHashSet<String>) -> bool {
    block
        .statements
        .iter()
        .any(|s| stmt_references_store_name(s, store_names))
}

/// Complete expression walker — covers every ExprIR variant so the prologue
/// check matches exactly what the AST rename touches (CRITICAL fix).
fn expr_references_store_name(expr: &ExprIR, store_names: &FxHashSet<String>) -> bool {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => store_names.contains(name.trim_start_matches('$')),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_references_store_name(left, store_names)
                || expr_references_store_name(right, store_names)
        }
        ExprIR::UnaryOpExpr { operand, .. } => expr_references_store_name(operand, store_names),
        ExprIR::CallExpr { args, .. } => args
            .iter()
            .any(|a| expr_references_store_name(a, store_names)),
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_references_store_name(object, store_names)
                || args
                    .iter()
                    .any(|a| expr_references_store_name(a, store_names))
        }
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            expr_references_store_name(object, store_names)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_references_store_name(object, store_names)
                || expr_references_store_name(index, store_names)
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_references_store_name(cond, store_names)
                || expr_references_store_name(then_expr, store_names)
                || expr_references_store_name(else_expr, store_names)
        }
        ExprIR::ArrayExpr { elements } => elements
            .iter()
            .any(|e| expr_references_store_name(e, store_names)),
        ExprIR::MapExpr { entries } => entries.iter().any(|(k, v)| {
            expr_references_store_name(k, store_names) || expr_references_store_name(v, store_names)
        }),
        ExprIR::ClosureExpr { body, .. } => expr_references_store_name(body, store_names),
        ExprIR::BlockExpr { statements, tail } => {
            statements
                .iter()
                .any(|s| stmt_references_store_name(s, store_names))
                || expr_references_store_name(tail, store_names)
        }
        ExprIR::MatchExpr { matched, arms } => {
            expr_references_store_name(matched, store_names)
                || arms
                    .iter()
                    .any(|arm| block_references_store_name(&arm.body, store_names))
        }
        ExprIR::VariantLiteral { fields, .. } | ExprIR::StructExpr { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_references_store_name(v, store_names)),
        ExprIR::SpreadExpr { operand } => expr_references_store_name(operand, store_names),
        ExprIR::StoreFieldAccessExpr { call_args, .. } => call_args
            .iter()
            .any(|a| expr_references_store_name(a, store_names)),
        ExprIR::AwaitExpr { operand } => expr_references_store_name(operand, store_names),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            expr_references_store_name(subject, store_names)
                || expr_references_store_name(predicate, store_names)
        }
        ExprIR::RefOpExpr { op, .. } => ref_op_references_store_name(op, store_names),
        ExprIR::LiteralExpr { .. } => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

/// Walk RefOpIR arguments for store-name references.
fn ref_op_references_store_name(op: &RefOpIR, store_names: &FxHashSet<String>) -> bool {
    match op {
        RefOpIR::Focus
        | RefOpIR::Blur
        | RefOpIR::Measure { .. }
        | RefOpIR::Select
        | RefOpIR::Value => false,
        RefOpIR::ScrollTo { x, y } => {
            x.as_ref()
                .is_some_and(|e| expr_references_store_name(e, store_names))
                || y.as_ref()
                    .is_some_and(|e| expr_references_store_name(e, store_names))
        }
        RefOpIR::ScrollBy { dx, dy } => {
            dx.as_ref()
                .is_some_and(|e| expr_references_store_name(e, store_names))
                || dy
                    .as_ref()
                    .is_some_and(|e| expr_references_store_name(e, store_names))
        }
        RefOpIR::SetSelection { start, end } => {
            expr_references_store_name(start, store_names)
                || expr_references_store_name(end, store_names)
        }
        RefOpIR::CapturePointer | RefOpIR::ReleasePointer => false,
        // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Emit a statement list, coalescing consecutive store-signal mutations
/// into ONE `set({ ... })` call (Task 3.5 M4): a multi-field action update
/// triggers a single Zustand subscriber notification instead of one per
/// write. The run breaks at non-mutation statements, bare (non-store)
/// assignments, and a repeated target — duplicate keys in one object
/// literal would silently drop the earlier write.
fn gen_zustand_action_stmts(
    w: &mut CodeWriter,
    stmts: &[StatementIR],
    renames: &FxHashMap<String, String>,
    set_fn: &str,
) {
    gen_zustand_action_stmts_inner(w, stmts, renames, true, set_fn);
}

/// Inner statement loop. `tail_returns` is true ONLY for the top-level
/// action body: a trailing expression statement there is the action's
/// return value (`return <expr>;`, H1 — same silent-value-drop fix as
/// component fns). Nested blocks (if/while/for/try bodies) never return
/// their last expression — a `return` inside them would exit the action.
///
/// `set_fn` is the Zustand setter callable to emit — `"set"` for stores
/// without getters, `"__set"` for stores with the getter-recompute wrapper
/// (Phase 5.2).
fn gen_zustand_action_stmts_inner(
    w: &mut CodeWriter,
    stmts: &[StatementIR],
    renames: &FxHashMap<String, String>,
    tail_returns: bool,
    set_fn: &str,
) {
    let mut i = 0;
    while i < stmts.len() {
        if let StatementIR::MutationStmt {
            intent: Some(intent),
        } = &stmts[i]
        {
            if let Some(first_part) = format_zustand_mutation_part(intent, renames) {
                let mut parts: Vec<String> = vec![first_part];
                let mut targets: FxHashSet<String> = FxHashSet::default();
                targets.insert(mutation_target_name(intent));
                let mut j = i + 1;
                while j < stmts.len() {
                    if let StatementIR::MutationStmt { intent: Some(next) } = &stmts[j] {
                        let next_target = mutation_target_name(next);
                        if targets.contains(&next_target) {
                            break;
                        }
                        if let Some(part) = format_zustand_mutation_part(next, renames) {
                            targets.insert(next_target);
                            parts.push(part);
                            j += 1;
                            continue;
                        }
                    }
                    break;
                }
                w.emit_fmt(format_args!("{}({{ {} }});", set_fn, parts.join(", ")));
                i = j;
                continue;
            }
        }
        if tail_returns && i + 1 == stmts.len() {
            if let StatementIR::ExprStmtIR { expr } = &stmts[i] {
                let line = expr::format_expr_ir_tsx_renamed(expr, &FxHashSet::default(), renames);
                w.emit_fmt(format_args!("return {};", line));
                i += 1;
                continue;
            }
        }
        gen_zustand_action_stmt(w, &stmts[i], renames, set_fn);
        i += 1;
    }
}

/// The signal name a mutation writes — used to keep coalesced runs free of
/// duplicate keys.
fn mutation_target_name(intent: &MutationIntent) -> String {
    match intent {
        MutationIntent::DirectAssign { target, .. }
        | MutationIntent::Increment { target, .. }
        | MutationIntent::Decrement { target, .. }
        | MutationIntent::Multiply { target, .. }
        | MutationIntent::Divide { target, .. }
        | MutationIntent::Modulo { target, .. }
        | MutationIntent::Push { target, .. }
        | MutationIntent::Pop { target } => target.name.clone(),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    }
}

/// Format one store-signal mutation as a `key: value` pair for a coalesced
/// `set({ ... })` call; `None` for bare (non-store) assignments, which must
/// stay plain statements outside the merge run.
fn format_zustand_mutation_part(
    intent: &MutationIntent,
    renames: &FxHashMap<String, String>,
) -> Option<String> {
    let fmt = |e: &ExprIR| expr::format_expr_ir_tsx_renamed(e, &FxHashSet::default(), renames);
    let (target, value_str) = match intent {
        MutationIntent::DirectAssign { target, value, .. } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            let val = value
                .as_ref()
                .map(|v| fmt(v))
                .unwrap_or_else(|| "null".into());
            (target, val)
        }
        MutationIntent::Increment { target, amount } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            let amt = amount
                .as_ref()
                .map(|v| fmt(v))
                .unwrap_or_else(|| "0".into());
            (target, format!("state.{} + {}", target.name, amt))
        }
        MutationIntent::Decrement { target, amount } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            let amt = amount
                .as_ref()
                .map(|v| fmt(v))
                .unwrap_or_else(|| "0".into());
            (target, format!("state.{} - {}", target.name, amt))
        }
        MutationIntent::Multiply { target, amount } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            let amt = amount
                .as_ref()
                .map(|v| fmt(v))
                .unwrap_or_else(|| "1".into());
            (target, format!("state.{} * {}", target.name, amt))
        }
        MutationIntent::Divide { target, amount } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            let amt = amount
                .as_ref()
                .map(|v| fmt(v))
                .unwrap_or_else(|| "1".into());
            (target, format!("state.{} / {}", target.name, amt))
        }
        MutationIntent::Modulo { target, amount } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            let amt = amount
                .as_ref()
                .map(|v| fmt(v))
                .unwrap_or_else(|| "1".into());
            (target, format!("state.{} % {}", target.name, amt))
        }
        MutationIntent::Push { target, item } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            let it = item
                .as_ref()
                .map(|v| fmt(v))
                .unwrap_or_else(|| "undefined".into());
            (target, format!("[...state.{}, {}]", target.name, it))
        }
        MutationIntent::Pop { target } => {
            if !renames.contains_key(target.name.as_str()) {
                return None;
            }
            (target, format!("state.{}.slice(0, -1)", target.name))
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    };
    Some(format!("{}: {}", target.name, value_str))
}

/// Format a single mutation into a Zustand `set()` call.
/// `renames` maps store signals/getters to `state.X` — applied to all value
/// expressions at the AST level (C8). `set_fn` is the setter to emit
/// (`"set"`, or `"__set"` for stores with the getter-recompute wrapper,
/// Phase 5.2).
fn format_zustand_mutation(
    w: &mut CodeWriter,
    intent: &MutationIntent,
    renames: &FxHashMap<String, String>,
    set_fn: &str,
) {
    match intent {
        MutationIntent::DirectAssign { target, value, .. } => {
            let val = value
                .as_ref()
                .map(|v| expr::format_expr_ir_tsx_renamed(v, &FxHashSet::default(), renames))
                .unwrap_or_else(|| "null".into());
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!("{}({{ {}: {} }});", set_fn, target_str, val));
            } else {
                w.emit_fmt(format_args!("{} = {};", target_str, val));
            }
        }
        MutationIntent::Increment { target, amount } => {
            let amt = amount
                .as_ref()
                .map(|v| expr::format_expr_ir_tsx_renamed(v, &FxHashSet::default(), renames))
                .unwrap_or_else(|| "0".into());
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "{}({{ {}: state.{} + {} }});",
                    set_fn, target_str, target_str, amt
                ));
            } else {
                w.emit_fmt(format_args!("{} += {};", target_str, amt));
            }
        }
        MutationIntent::Decrement { target, amount } => {
            let amt = amount
                .as_ref()
                .map(|v| expr::format_expr_ir_tsx_renamed(v, &FxHashSet::default(), renames))
                .unwrap_or_else(|| "0".into());
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "{}({{ {}: state.{} - {} }});",
                    set_fn, target_str, target_str, amt
                ));
            } else {
                w.emit_fmt(format_args!("{} -= {};", target_str, amt));
            }
        }
        MutationIntent::Push { target, item } => {
            let it = item
                .as_ref()
                .map(|v| expr::format_expr_ir_tsx_renamed(v, &FxHashSet::default(), renames))
                .unwrap_or_else(|| "undefined".into());
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "{}({{ {}: [...state.{}, {}] }});",
                    set_fn, target_str, target_str, it
                ));
            } else {
                w.emit_fmt(format_args!(
                    "{} = [...{}, {}];",
                    target_str, target_str, it
                ));
            }
        }
        MutationIntent::Pop { target } => {
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "{}({{ {}: state.{}.slice(0, -1) }});",
                    set_fn, target_str, target_str
                ));
            } else {
                w.emit_fmt(format_args!(
                    "{} = {}.slice(0, -1);",
                    target_str, target_str
                ));
            }
        }
        MutationIntent::Multiply { target, amount } => {
            let amt = amount
                .as_ref()
                .map(|v| expr::format_expr_ir_tsx_renamed(v, &FxHashSet::default(), renames))
                .unwrap_or_else(|| "1".into());
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "{}({{ {}: state.{} * {} }});",
                    set_fn, target_str, target_str, amt
                ));
            } else {
                w.emit_fmt(format_args!("{} *= {};", target_str, amt));
            }
        }
        MutationIntent::Divide { target, amount } => {
            let amt = amount
                .as_ref()
                .map(|v| expr::format_expr_ir_tsx_renamed(v, &FxHashSet::default(), renames))
                .unwrap_or_else(|| "1".into());
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "{}({{ {}: state.{} / {} }});",
                    set_fn, target_str, target_str, amt
                ));
            } else {
                w.emit_fmt(format_args!("{} /= {};", target_str, amt));
            }
        }
        MutationIntent::Modulo { target, amount } => {
            let amt = amount
                .as_ref()
                .map(|v| expr::format_expr_ir_tsx_renamed(v, &FxHashSet::default(), renames))
                .unwrap_or_else(|| "1".into());
            let target_str = &target.name;
            if renames.contains_key(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "{}({{ {}: state.{} % {} }});",
                    set_fn, target_str, target_str, amt
                ));
            } else {
                w.emit_fmt(format_args!("{} %= {};", target_str, amt));
            }
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    }
}

/// Escape JavaScript reserved words that cannot be used as parameter/variable names.
pub(super) fn escape_js_ident(name: &str) -> String {
    let reserved = [
        "abstract",
        "arguments",
        "await",
        "boolean",
        "break",
        "byte",
        "case",
        "catch",
        "char",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "double",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "final",
        "finally",
        "float",
        "for",
        "function",
        "goto",
        "if",
        "implements",
        "import",
        "in",
        "instanceof",
        "int",
        "interface",
        "let",
        "long",
        "native",
        "new",
        "null",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "short",
        "static",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "true",
        "try",
        "typeof",
        "var",
        "void",
        "volatile",
        "while",
        "with",
        "yield",
    ];
    if reserved.contains(&name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
}
