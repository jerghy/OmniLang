//! React codegen: IR -> React TSX components + CSS modules + store hooks.
//!
//! Each component generates a `.tsx` React functional component with hooks
//! and a `.module.css` CSS module file. Each store generates a `useXxx.ts`
//! custom hook file.

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::generator::build_component_map;
use crate::codegen::shared::types::FormatType;
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

pub struct ReactCodegen;

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
        vec![CompiledFile {
            path: format!("{}use{}.ts", dir_prefix, name),
            content: self.gen_store_hook(name, store),
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
