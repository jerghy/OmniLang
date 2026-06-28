//! Codegen module -- multi-target code generation from Intent IR.
//!
//! Currently supported targets:
//! - OmniView (mature): JS/HTML/CSS with inline signal runtime
//! - Python, JavaScript, Rust, Dart (planned): pure-logic codegen
//! - React, Vue, Dioxus, Flutter (planned): framework-based codegen

pub mod dart;
pub mod dioxus;
pub mod flutter;
pub mod javascript;
pub mod omniview;
pub mod python;
pub mod react;
pub mod rust;
pub mod vue;

use std::collections::HashMap;

use crate::ir::IRProgram;

/// CodeWriter: tracks indentation and emits formatted lines.
#[derive(Debug)]
pub struct CodeWriter {
    lines: Vec<String>,
    indent_level: usize,
    indent_str: &'static str,
}

impl CodeWriter {
    pub fn new() -> Self {
        Self {
            lines: vec![],
            indent_level: 0,
            indent_str: "    ",
        }
    }

    /// Emit a line at the current indent level.
    /// An empty line emits a blank line (no indent).
    pub fn emit(&mut self, line: &str) {
        if line.is_empty() {
            self.lines.push(String::new());
        } else {
            let pad = self.indent_str.repeat(self.indent_level);
            self.lines.push(format!("{}{}", pad, line));
        }
    }

    /// Increase indentation by one level.
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation by one level (clamped at 0).
    pub fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    /// Finalize the output string (lines joined by newline, trailing newline).
    pub fn into_string(self) -> String {
        self.lines.join("\n") + "\n"
    }
}

impl Default for CodeWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// A single compiled output file.
#[derive(Debug, Clone)]
pub struct CompiledFile {
    pub path: String,
    pub content: String,
}

/// Result of code generation for a target.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub files: Vec<CompiledFile>,
    pub errors: Vec<String>,
}

impl CompileOutput {
    /// Build a successful output with the given files.
    pub fn success(files: Vec<CompiledFile>) -> Self {
        Self {
            files,
            errors: vec![],
        }
    }

    /// Build an error output with a message.
    pub fn error(msg: String) -> Self {
        Self {
            files: vec![],
            errors: vec![msg],
        }
    }
}

/// Code generation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Dart,
    React,
    Vue,
    Dioxus,
    Flutter,
    OmniView,
}

impl Target {
    /// Return the file extension for this target.
    pub fn extension(&self) -> &'static str {
        match self {
            Target::Python => ".py",
            Target::JavaScript => ".js",
            Target::TypeScript => ".ts",
            Target::Rust => ".rs",
            Target::Dart => ".dart",
            Target::React => ".tsx",
            Target::Vue => ".vue",
            Target::Dioxus => ".rs",
            Target::Flutter => ".dart",
            Target::OmniView => ".js",
        }
    }
}

impl std::str::FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Ok(Target::Python),
            "javascript" | "js" => Ok(Target::JavaScript),
            "typescript" | "ts" => Ok(Target::TypeScript),
            "rust" | "rs" => Ok(Target::Rust),
            "dart" => Ok(Target::Dart),
            "react" | "reactjs" | "react.js" => Ok(Target::React),
            "vue" | "vuejs" | "vue.js" => Ok(Target::Vue),
            "dioxus" => Ok(Target::Dioxus),
            "flutter" => Ok(Target::Flutter),
            "omniview" => Ok(Target::OmniView),
            _ => Err(format!("unknown target: '{s}'")),
        }
    }
}

/// Codegen trait -- implement for each target.
pub trait Codegen {
    /// Generate output files for the given IR program and source files.
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput;
}

/// Sanitize a CSS value string to prevent injection attacks.
///
/// Rejects dangerous CSS value patterns that could execute JavaScript:
/// `javascript:`, `expression(`, `-moz-binding`, `data:text/html`.
/// Returns the original string if safe, or an empty string with a warning.
pub fn sanitize_css_value(value: &str) -> String {
    let lower = value.to_lowercase();
    if lower.contains("javascript:")
        || lower.contains("expression(")
        || lower.contains("-moz-binding")
        || lower.contains("data:text/html")
    {
        // Return a safe fallback — the dangerous value is suppressed
        return String::new();
    }
    value.to_string()
}

/// Sanitize a module name for use as a file path component.
///
/// Strips directory traversal sequences (`..`, `/`, `\`) and leading dots
/// to prevent path traversal when constructing output file paths.
/// The actual file I/O is handled by the Python CLI; this ensures
/// generated paths are safe regardless.
pub fn sanitize_module_name(name: &str) -> String {
    let sanitized = name
        .strip_suffix(".omni")
        .unwrap_or(name)
        .replace("..", "_")
        .replace(['/', '\\', ':'], "_")
        .trim_start_matches('.')
        .to_string();
    // Guard against empty result (e.g., name was just "." or all dots)
    if sanitized.is_empty() {
        "module".to_string()
    } else {
        sanitized
    }
}

/// Sanitize an identifier for use in JavaScript/TypeScript output.
///
/// Appends `_` to names that collide with JavaScript reserved keywords
/// (e.g. `default` → `default_`, `class` → `class_`).
/// This prevents syntax errors when OmniLang identifiers use names
/// that are valid in Omni but reserved in JavaScript.
pub fn sanitize_js_ident(name: &str) -> String {
    // JavaScript reserved words (ES5+ strict mode) that cannot be used as identifiers
    const JS_RESERVED: &[&str] = &[
        "break",
        "case",
        "catch",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "finally",
        "for",
        "function",
        "if",
        "in",
        "instanceof",
        "new",
        "return",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        // ES6+
        "class",
        "const",
        "enum",
        "export",
        "extends",
        "import",
        "super",
        "implements",
        "interface",
        "let",
        "package",
        "private",
        "protected",
        "public",
        "static",
        "yield",
        // ES2017+
        "async",
        "await",
        // Always reserved as literals / special bindings
        "arguments",
        "null",
        "true",
        "false",
    ];

    if JS_RESERVED.contains(&name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
}

/// Compile IR to the specified target.
///
/// Routes to each target's codegen module. Framework targets and
/// stub modules return error until implemented.
pub fn compile(
    ir: &IRProgram,
    target: Target,
    source_files: &HashMap<String, String>,
) -> CompileOutput {
    // Validate stdlib imports for the target
    if let Err(errors) = validate_stdlib_imports(ir, target) {
        return CompileOutput {
            files: vec![],
            errors,
        };
    }
    match target {
        Target::OmniView => omniview::compile_files(ir, source_files),
        Target::Python => python::PythonCodegen.generate(ir, source_files),
        Target::JavaScript => javascript::JavaScriptCodegen::new(false).generate(ir, source_files),
        Target::TypeScript => javascript::JavaScriptCodegen::new(true).generate(ir, source_files),
        Target::Rust => rust::RustCodegen.generate(ir, source_files),
        Target::Dart => dart::DartCodegen.generate(ir, source_files),
        Target::React => react::ReactCodegen.generate(ir, source_files),
        Target::Vue => vue::VueCodegen.generate(ir, source_files),
        Target::Dioxus => dioxus::DioxusCodegen.generate(ir, source_files),
        Target::Flutter => flutter::FlutterCodegen.generate(ir, source_files),
    }
}

/// Try to format a stdlib function call. Returns `Some(code)` if the call
/// matches a registered stdlib function, `None` otherwise (so backends fall
/// through to their normal CallExpr handling).
pub fn try_format_stdlib_call(name: &str, args: &[String], target: Target) -> Option<String> {
    // Check if name looks like "module_fn" stdlib pattern
    if let Some((module, fn_name)) = name.split_once('_') {
        if let Some(entry) = crate::stdlib::lookup(module, fn_name) {
            if entry.available_targets.contains(&target) {
                return Some((entry.codegen)(args, target));
            }
        }
    }
    // Also try direct lookup (for non-prefixed names)
    // Stdlib functions are imported as simple names, and lowered as-is
    for module in &[
        "http", "json", "storage", "math", "string", "array", "console",
    ] {
        if let Some(entry) = crate::stdlib::lookup(module, name) {
            if entry.available_targets.contains(&target) {
                return Some((entry.codegen)(args, target));
            }
        }
    }
    None
}

/// Validate that all stdlib imports in the IR are available on the given target.
pub fn validate_stdlib_imports(
    ir: &crate::ir::IRProgram,
    target: Target,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for module in &ir.stdlib_imports {
        if !crate::stdlib::module_available(module, target) {
            let available = crate::stdlib::available_targets_for(module);
            let available_str: Vec<String> = available
                .iter()
                .map(|t| format!("{:?}", t).to_lowercase())
                .collect();
            errors.push(format!(
                "stdlib module 'std/{}' is not available on target '{:?}'. Available targets: {}",
                module,
                target,
                available_str.join(", ")
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_target_parse_all() {
        assert_eq!(Target::from_str("python").unwrap(), Target::Python);
        assert_eq!(Target::from_str("py").unwrap(), Target::Python);
        assert_eq!(Target::from_str("javascript").unwrap(), Target::JavaScript);
        assert_eq!(Target::from_str("js").unwrap(), Target::JavaScript);
        assert_eq!(Target::from_str("typescript").unwrap(), Target::TypeScript);
        assert_eq!(Target::from_str("ts").unwrap(), Target::TypeScript);
        assert_eq!(Target::from_str("rust").unwrap(), Target::Rust);
        assert_eq!(Target::from_str("rs").unwrap(), Target::Rust);
        assert_eq!(Target::from_str("dart").unwrap(), Target::Dart);
        assert_eq!(Target::from_str("react").unwrap(), Target::React);
        assert_eq!(Target::from_str("vue").unwrap(), Target::Vue);
        assert_eq!(Target::from_str("dioxus").unwrap(), Target::Dioxus);
        assert_eq!(Target::from_str("flutter").unwrap(), Target::Flutter);
        assert_eq!(Target::from_str("omniview").unwrap(), Target::OmniView);
        assert!(Target::from_str("unknown").is_err());
    }

    #[test]
    fn test_target_extension() {
        assert_eq!(Target::Python.extension(), ".py");
        assert_eq!(Target::JavaScript.extension(), ".js");
        assert_eq!(Target::TypeScript.extension(), ".ts");
        assert_eq!(Target::Rust.extension(), ".rs");
        assert_eq!(Target::Dart.extension(), ".dart");
        assert_eq!(Target::React.extension(), ".tsx");
        assert_eq!(Target::Vue.extension(), ".vue");
        assert_eq!(Target::Dioxus.extension(), ".rs");
        assert_eq!(Target::Flutter.extension(), ".dart");
        assert_eq!(Target::OmniView.extension(), ".js");
    }

    #[test]
    fn test_code_writer() {
        let mut w = CodeWriter::new();
        w.emit("fn main() {");
        w.indent();
        w.emit("println!(\"hello\");");
        w.dedent();
        w.emit("}");
        let out = w.into_string();
        assert_eq!(out, "fn main() {\n    println!(\"hello\");\n}\n");
    }

    // ── Stdlib integration tests ───────────────────────

    #[test]
    fn test_try_format_stdlib_call_http_get_js() {
        let result = try_format_stdlib_call("http_get", &["\"url\"".into()], Target::JavaScript);
        assert!(result.is_some(), "http_get should be recognized");
        assert!(
            result.unwrap().contains("fetch"),
            "JS http_get should use fetch"
        );
    }

    #[test]
    fn test_try_format_stdlib_call_json_parse_js() {
        let result = try_format_stdlib_call("json_parse", &["text".into()], Target::JavaScript);
        assert!(result.is_some(), "json_parse should be recognized");
        assert_eq!(result.unwrap(), "JSON.parse(text)");
    }

    #[test]
    fn test_try_format_stdlib_call_json_stringify_python() {
        let result = try_format_stdlib_call("json_stringify", &["obj".into()], Target::Python);
        assert!(result.is_some(), "json_stringify should be recognized");
        assert_eq!(result.unwrap(), "json.dumps(obj)");
    }

    #[test]
    fn test_try_format_stdlib_call_not_found() {
        let result = try_format_stdlib_call("nonexistent_fn", &[], Target::JavaScript);
        assert!(result.is_none(), "Unknown fn should return None");
    }

    #[test]
    fn test_try_format_stdlib_call_math_abs_js() {
        let result = try_format_stdlib_call("math_abs", &["x".into()], Target::JavaScript);
        assert!(result.is_some(), "math_abs should be recognized");
        assert_eq!(result.unwrap(), "Math.abs(x)");
    }

    #[test]
    fn test_try_format_stdlib_call_console_log_js() {
        let result = try_format_stdlib_call("console_log", &["\"hi\"".into()], Target::JavaScript);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "console.log(\"hi\")");
    }

    #[test]
    fn test_validate_stdlib_imports_storage_on_rust_fails() {
        let ir = crate::ir::IRProgram {
            version: "1.0".into(),
            modules: HashMap::new(),
            source_map: HashMap::new(),
            stdlib_imports: vec!["storage".into()],
        };
        let result = validate_stdlib_imports(&ir, Target::Rust);
        assert!(result.is_err(), "storage on rust should fail");
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].contains("not available"),
            "Error should mention not available. Got: {}",
            errors[0]
        );
    }

    #[test]
    fn test_validate_stdlib_imports_all_valid() {
        let ir = crate::ir::IRProgram {
            version: "1.0".into(),
            modules: HashMap::new(),
            source_map: HashMap::new(),
            stdlib_imports: vec!["http".into(), "json".into()],
        };
        let result = validate_stdlib_imports(&ir, Target::JavaScript);
        assert!(
            result.is_ok(),
            "http+json on JS should be valid. Got: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_stdlib_imports_empty() {
        let ir = crate::ir::IRProgram {
            version: "1.0".into(),
            modules: HashMap::new(),
            source_map: HashMap::new(),
            stdlib_imports: vec![],
        };
        let result = validate_stdlib_imports(&ir, Target::Rust);
        assert!(result.is_ok(), "Empty imports should always be valid");
    }
}
