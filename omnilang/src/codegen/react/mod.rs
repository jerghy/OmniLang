//! React codegen: IR -> React TSX components + CSS modules + store hooks.
//!
//! Each component generates a `.tsx` React functional component with hooks
//! and a `.module.css` CSS module file. Each store generates a `useXxx.ts`
//! custom hook file.

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::generator::build_component_map;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::shared::ui_codegen::UiCodegen;
use crate::codegen::{format_json_value, JSON_REACT};
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

pub(super) mod component;
pub(super) mod expr;
pub(super) mod motion;
#[cfg(test)]
pub(super) mod tests;
pub(super) mod utils;
pub(crate) mod view;

/// How stores are generated for React targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ReactStoreMode {
    /// Generate custom React hooks with useState/useCallback/useMemo.
    #[default]
    Hooks,
    /// Generate Zustand stores using `create()` from zustand.
    Zustand,
}

impl std::str::FromStr for ReactStoreMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hooks" => Ok(Self::Hooks),
            "zustand" => Ok(Self::Zustand),
            _ => Err(format!(
                "unknown store mode: '{s}'. Expected 'hooks' or 'zustand'"
            )),
        }
    }
}


pub struct ReactCodegen {
    pub store_mode: ReactStoreMode,
}

impl Default for ReactCodegen {
    fn default() -> Self {
        Self {
            store_mode: ReactStoreMode::Hooks,
        }
    }
}

impl Codegen for ReactCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &FxHashMap<String, String>) -> CompileOutput {
        self.generate_per_component(ir, source_files)
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
    fn gen_store_file(&self, name: &str, store: &StoreIR) -> Vec<CompiledFile> {
        let dir_prefix = source_dir_prefix(store.source_file.as_deref());
        let content = match self.store_mode {
            ReactStoreMode::Hooks => self.gen_store_hook(name, store),
            ReactStoreMode::Zustand => self.gen_zustand_store(name, store),
        };
        vec![CompiledFile {
            path: format!("{}use{}.ts", dir_prefix, name),
            content,
        }]
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
    /// Generate a custom store hook (useStoreName.ts).
    fn gen_store_hook(&self, name: &str, store: &StoreIR) -> String {
        let mut w = CodeWriter::new();
        w.emit_fmt(format_args!(
            "// Generated by OmniLang — Store hook: use{}",
            name
        ));
        w.emit("import { useState, useCallback, useMemo } from 'react';");
        w.emit("");

        w.emit_fmt(format_args!("export function use{}() {{", name));
        w.indent();

        // Build a set of signal setter names to detect conflicts
        let signal_setter_names: FxHashSet<String> = store
            .signals
            .keys()
            .map(|s| format!("set{}", super::capitalize_first(s)))
            .collect();

        // Signals as state variables
        for (sig_name, sig_type) in &store.signals {
            let init = store
                .signal_defaults
                .get(sig_name)
                .map(|v| format_json_value(v, &JSON_REACT))
                .unwrap_or_else(|| "null".into());
            let setter = format!("set{}", super::capitalize_first(sig_name));
            let ts_type = crate::codegen::shared::types::JsTypeFormatter
                .format_type(sig_type.kind.as_str(), &sig_type.type_args);
            w.emit_fmt(format_args!(
                "const [{}, {}] = useState<{}>({});",
                sig_name, setter, ts_type, init
            ));
        }

        // Getters as memoized values (useMemo returns the value directly, not a function)
        for (getter_name, getter_expr) in &store.getters {
            let deps: Vec<String> = store
                .signals
                .keys()
                .filter(|sig| utils::getter_expr_contains_ident(getter_expr, sig))
                .cloned()
                .collect();
            let deps_str = deps.join(", ");
            w.emit_fmt(format_args!(
                "const {} = useMemo(() => {{ return {}; }}, [{}]);",
                getter_name,
                expr::format_expr_ir_tsx(getter_expr),
                deps_str
            ));
        }

        // Build set of store signal names so mutations use setter calls
        let store_signal_names: FxHashSet<String> = store.signals.keys().cloned().collect();

        // Build all store names (signals + getters) for dependency analysis
        let all_store_names: FxHashSet<String> = store
            .signals
            .keys()
            .chain(store.getters.keys())
            .cloned()
            .collect();

        // Actions: each becomes a useCallback function calling signal setters
        // with precise dependency arrays to avoid stale closures.
        let mut return_keys: Vec<String> = Vec::new();
        for action in &store.actions {
            let params: Vec<String> = action
                .params
                .iter()
                .map(|p| escape_js_ident(&p.name))
                .collect();
            let params_str = params.join(", ");
            // If the action name conflicts with a signal setter, rename the local variable
            let local_action_name = if signal_setter_names.contains(&action.name) {
                format!("__act{}", super::capitalize_first(&action.name))
            } else {
                action.name.clone()
            };
            w.emit_fmt(format_args!(
                "const {} = useCallback(({}) => {{",
                local_action_name, params_str
            ));
            w.indent();
            for stmt in &action.body.statements {
                expr::generate_statement_tsx(&mut w, stmt, &store_signal_names);
            }
            w.dedent();
            // Compute precise dependency array
            let action_deps: Vec<String> = all_store_names
                .iter()
                .filter(|name| utils::action_body_contains_ident(action, name))
                .cloned()
                .collect();
            if action_deps.is_empty() {
                w.emit("}, []);");
            } else {
                w.emit_fmt(format_args!("}}, [{}]);", action_deps.join(", ")));
            }
            // Return key is always the original action name; value may differ
            if local_action_name != action.name {
                return_keys.push(format!("{}: {}", action.name, local_action_name));
            } else {
                return_keys.push(action.name.clone());
            }
        }

        // Return value
        for sig_name in store.signals.keys() {
            return_keys.push(sig_name.clone());
        }
        for getter_name in store.getters.keys() {
            return_keys.push(getter_name.clone());
        }
        w.emit_fmt(format_args!("return {{ {} }};", return_keys.join(", ")));

        w.dedent();
        w.emit("}");
        w.into_string()
    }

    /// Generate a Zustand store (useStoreName.ts).
    fn gen_zustand_store(&self, name: &str, store: &StoreIR) -> String {
        let mut w = CodeWriter::new();
        w.emit_fmt(format_args!(
            "// Generated by OmniLang — Zustand store: use{}",
            name
        ));
        w.emit("import { create } from 'zustand';");
        w.emit("");

        // State interface
        let state_interface = format!("{}State", name);
        w.emit_fmt(format_args!("interface {} {{", state_interface));

        for (sig_name, sig_type) in &store.signals {
            let ts_type = JsTypeFormatter.format_type(sig_type.kind.as_str(), &sig_type.type_args);
            w.emit_fmt(format_args!("  {}: {};", sig_name, ts_type));
        }
        for getter_name in store.getters.keys() {
            w.emit_fmt(format_args!("  readonly {}: any;", getter_name));
        }
        for action in &store.actions {
            let params: Vec<String> = action
                .params
                .iter()
                .map(|p| format!("{}: any", escape_js_ident(&p.name)))
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

        // Collect store signal names for needs_get check
        let store_signal_names: FxHashSet<String> = store.signals.keys().cloned().collect();

        // Check if any getter or action body needs get()
        let needs_get = !store.getters.is_empty()
            || store
                .actions
                .iter()
                .any(|a| block_needs_zustand_state(&a.body, &store_signal_names));
        let create_params = if needs_get { "(set, get)" } else { "(set)" };

        // create() call
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

        // Getters as accessor properties — use get() for signal references
        for (getter_name, getter_expr) in &store.getters {
            let expr_str = expr::format_expr_ir_tsx(getter_expr);
            let all_signal_names: FxHashSet<String> = store.signals.keys().cloned().collect();
            let rewritten = rewrite_signal_refs_for_zustand(&expr_str, &all_signal_names);
            // The rewrite produces state.signalName, but in getter context
            // we need get().signalName (since getter is in object literal, not action body)
            let final_expr = rewritten.replace("state.", "get().");
            w.emit_fmt(format_args!(
                "get {}() {{ return {}; }},",
                getter_name, final_expr
            ));
        }

        // Actions
        for action in &store.actions {
            let params: Vec<String> = action
                .params
                .iter()
                .map(|p| escape_js_ident(&p.name))
                .collect();
            let params_str = params.join(", ");
            w.emit_fmt(format_args!("{}: ({}) => {{", action.name, params_str));
            w.indent();

            // Emit const state = get(); only if the action body references store signals
            emit_zustand_action_prologue(&mut w, action, &store_signal_names);

            for stmt in &action.body.statements {
                gen_zustand_action_stmt(&mut w, stmt, &store_signal_names);
            }
            w.dedent();
            w.emit("},");
        }

        w.dedent();
        w.emit("}));");
        w.into_string()
    }
}

/// Generate a statement inside a Zustand store action body.
/// Mutations to store signals are wrapped in `set((state) => (...))`.
fn gen_zustand_action_stmt(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
) {
    match stmt {
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                format_zustand_mutation(w, intent, signal_names);
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = expr::format_expr_ir_tsx(expr);
            w.emit_fmt(format_args!("{};", line));
        }
        StatementIR::LetStmtIR {
            name,
            value,
            r#type: _,
        } => {
            let rhs = value
                .as_ref()
                .map(expr::format_expr_ir_tsx)
                .unwrap_or_else(|| "null".into());
            w.emit_fmt(format_args!("const {} = {};", name, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = expr::format_expr_ir_tsx(condition);
            w.emit_fmt(format_args!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                gen_zustand_action_stmt(w, s, signal_names);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif = expr::format_expr_ir_tsx(elif_cond);
                w.emit_fmt(format_args!("}} else if ({}) {{", elif));
                w.indent();
                for s in &elif_body.statements {
                    gen_zustand_action_stmt(w, s, signal_names);
                }
                w.dedent();
            }
            if let Some(ref eb) = else_body {
                w.emit("} else {");
                w.indent();
                for s in &eb.statements {
                    gen_zustand_action_stmt(w, s, signal_names);
                }
                w.dedent();
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let v = value
                .as_ref()
                .map(expr::format_expr_ir_tsx)
                .unwrap_or_else(|| "null".into());
            w.emit_fmt(format_args!("return {};", v));
        }
        _ => {
            // Fallback: use default TSX statement generation for non-mutation stmts
            expr::generate_statement_tsx(w, stmt, signal_names);
        }
    }
}

/// Rewrite signal-name references in a formatted expression string to use `state.` prefix.
/// Each Zustand action body is wrapped with `const state = get();` so signal references
/// become `state.items`, `state.count`, etc.
fn rewrite_signal_refs_for_zustand(expr_str: &str, signal_names: &FxHashSet<String>) -> String {
    // Sort by length descending to avoid partial replacements (e.g., "items" before "item")
    let mut names: Vec<&String> = signal_names.iter().collect();
    names.sort_by_key(|n| -(n.len() as i32));
    let mut result = expr_str.to_string();
    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_' || c == '$';

    for name in &names {
        let pattern = name.as_str();
        let replacement = format!("state.{}", pattern);
        // Simple string replacement for whole-word matches only
        let bytes = result.as_bytes();
        let pattern_bytes = pattern.as_bytes();
        let mut new_result = String::with_capacity(result.len());
        let mut i = 0;
        while i < bytes.len() {
            if i + pattern_bytes.len() <= bytes.len()
                && &bytes[i..i + pattern_bytes.len()] == pattern_bytes
            {
                let before = i == 0 || !is_ident_char(bytes[i - 1] as char);
                let after = i + pattern_bytes.len() >= bytes.len()
                    || !is_ident_char(bytes[i + pattern_bytes.len()] as char);
                if before && after {
                    new_result.push_str(&replacement);
                    i += pattern_bytes.len();
                    continue;
                }
            }
            new_result.push(bytes[i] as char);
            i += 1;
        }
        result = new_result;
    }
    result
}

/// Check if any statement in a block mutates any of the given store signal names.
fn block_mutates_store_signal(block: &BlockIR, signal_names: &FxHashSet<String>) -> bool {
    block
        .statements
        .iter()
        .any(|s| stmt_mutates_store_signal(s, signal_names))
}

fn stmt_mutates_store_signal(stmt: &StatementIR, signal_names: &FxHashSet<String>) -> bool {
    match stmt {
        StatementIR::MutationStmt {
            intent: Some(intent),
        } => {
            let target_name = match intent {
                MutationIntent::DirectAssign { target, .. }
                | MutationIntent::Increment { target, .. }
                | MutationIntent::Decrement { target, .. }
                | MutationIntent::Multiply { target, .. }
                | MutationIntent::Divide { target, .. }
                | MutationIntent::Modulo { target, .. }
                | MutationIntent::Push { target, .. }
                | MutationIntent::Pop { target } => &target.name,
            };
            signal_names.contains(target_name)
        }
        _ => false,
    }
}

/// Emit a `const state = get();` line for Zustand action body prologue.
/// Only emit when the action uses mutations that need `state.` access
/// (Increment, Decrement, Push, Pop, Multiply, Divide, Modulo).
/// DirectAssign uses `set({ key: val })` and does not need state.
fn emit_zustand_action_prologue(
    w: &mut CodeWriter,
    action: &FunctionIR,
    signal_names: &FxHashSet<String>,
) {
    if block_needs_zustand_state(&action.body, signal_names) {
        w.emit("const state = get();");
    }
}

/// Returns true if any statement in the block needs `state.` access in Zustand mutations.
/// DirectAssign uses `set({...})` (no state needed); all other compound mutations need state.
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
        } => {
            let target_name = match intent {
                MutationIntent::Increment { target, .. }
                | MutationIntent::Decrement { target, .. }
                | MutationIntent::Multiply { target, .. }
                | MutationIntent::Divide { target, .. }
                | MutationIntent::Modulo { target, .. }
                | MutationIntent::Push { target, .. }
                | MutationIntent::Pop { target } => &target.name,
                // DirectAssign uses set({}) and does not need state
                MutationIntent::DirectAssign { .. } => return false,
            };
            signal_names.contains(target_name)
        }
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
        _ => false,
    }
}

/// Format a single mutation into a Zustand `set()` call.
fn format_zustand_mutation(
    w: &mut CodeWriter,
    intent: &MutationIntent,
    signal_names: &FxHashSet<String>,
) {
    match intent {
        MutationIntent::DirectAssign { target, value } => {
            let val = value
                .as_ref()
                .map(|v| {
                    let s = expr::format_expr_ir_tsx(v);
                    rewrite_signal_refs_for_zustand(&s, signal_names)
                })
                .unwrap_or_else(|| "null".into());
            let target_str = &target.name;
            if signal_names.contains(target_str.as_str()) {
                w.emit_fmt(format_args!("set({{ {}: {} }});", target_str, val));
            } else {
                w.emit_fmt(format_args!("{} = {};", target_str, val));
            }
        }
        MutationIntent::Increment { target, amount } => {
            let amt = amount
                .as_ref()
                .map(|v| {
                    let s = expr::format_expr_ir_tsx(v);
                    rewrite_signal_refs_for_zustand(&s, signal_names)
                })
                .unwrap_or_else(|| "0".into());
            let target_str = &target.name;
            if signal_names.contains(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "set({{ {}: state.{} + {} }});",
                    target_str, target_str, amt
                ));
            } else {
                w.emit_fmt(format_args!("{} += {};", target_str, amt));
            }
        }
        MutationIntent::Decrement { target, amount } => {
            let amt = amount
                .as_ref()
                .map(|v| {
                    let s = expr::format_expr_ir_tsx(v);
                    rewrite_signal_refs_for_zustand(&s, signal_names)
                })
                .unwrap_or_else(|| "0".into());
            let target_str = &target.name;
            if signal_names.contains(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "set({{ {}: state.{} - {} }});",
                    target_str, target_str, amt
                ));
            } else {
                w.emit_fmt(format_args!("{} -= {};", target_str, amt));
            }
        }
        MutationIntent::Push { target, item } => {
            let it = item
                .as_ref()
                .map(|v| {
                    let s = expr::format_expr_ir_tsx(v);
                    rewrite_signal_refs_for_zustand(&s, signal_names)
                })
                .unwrap_or_else(|| "undefined".into());
            let target_str = &target.name;
            if signal_names.contains(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "set({{ {}: [...state.{}, {}] }});",
                    target_str, target_str, it
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
            if signal_names.contains(target_str.as_str()) {
                w.emit_fmt(format_args!(
                    "set({{ {}: state.{}.slice(0, -1) }});",
                    target_str, target_str
                ));
            } else {
                w.emit_fmt(format_args!(
                    "{} = {}.slice(0, -1);",
                    target_str, target_str
                ));
            }
        }
        _ => {
            // For Multiply, Divide, Modulo — use set() with get() access
            let _ = signal_names;
            w.emit("// TODO: zustand mutation");
        }
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

// ═══════════════════════════════════════════════════════════
// UiCodegen trait implementation — thin delegation to existing methods
// ═══════════════════════════════════════════════════════════

impl UiCodegen for ReactCodegen {
    fn generate_component_code(&self, name: &str, comp: &ComponentIR, ir: &IRProgram) -> String {
        let component_map = build_component_map(ir);
        self.gen_component_tsx(name, comp, ir, &component_map)
    }

    fn generate_component_css(&self, name: &str, comp: &ComponentIR) -> String {
        self.gen_component_css(name, comp)
    }

    fn generate_store_code(&self, name: &str, store: &StoreIR) -> String {
        match self.store_mode {
            ReactStoreMode::Hooks => self.gen_store_hook(name, store),
            ReactStoreMode::Zustand => self.gen_zustand_store(name, store),
        }
    }

    fn component_file_extension(&self) -> &str {
        ".tsx"
    }

    fn store_file_extension(&self) -> &str {
        ".ts"
    }
}
