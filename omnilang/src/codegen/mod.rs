//! Codegen module -- multi-target code generation from Intent IR.
//!
//! Currently supported targets:
//! - OmniView: JS/HTML/CSS with inline signal runtime
//! - Python, JavaScript, Rust, Dart: pure-logic codegen
//! - React, Vue, Dioxus, Flutter: framework-based codegen

#[cfg(feature = "codegen-dart")]
pub mod dart;
#[cfg(feature = "codegen-dioxus")]
pub mod dioxus;
#[cfg(feature = "codegen-flutter")]
pub mod flutter;
#[cfg(feature = "codegen-javascript")]
pub mod javascript;
#[cfg(feature = "codegen-omniview")]
pub mod omniview;
#[cfg(feature = "codegen-python")]
pub mod python;
#[cfg(feature = "codegen-react")]
pub mod react;
#[cfg(feature = "codegen-rust")]
pub mod rust;
#[cfg(feature = "codegen-vue")]
pub mod vue;

use std::collections::HashMap;
use std::fmt;

use crate::ir::EventDataKind;
use crate::ir::IRProgram;

/// CodeWriter: tracks indentation and emits formatted lines.
#[derive(Debug)]
pub struct CodeWriter {
    lines: Vec<String>,
    indent_level: usize,
    indent_str: &'static str,
}

impl CodeWriter {
    #[must_use]
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

    /// Emit a line with current indentation, using fmt::Arguments.
    /// More efficient than `emit(&format!(...))` — avoids intermediate String allocation.
    ///
    /// # Example
    /// ```ignore
    /// w.emit_fmt(format_args!("let {name}{ty} = {rhs};"));
    /// ```
    pub fn emit_fmt(&mut self, args: fmt::Arguments<'_>) {
        let indent = self.indent_str.repeat(self.indent_level);
        let mut line = indent;
        // write! to String never fails
        let _ = fmt::write(&mut line, args);
        self.lines.push(line);
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
    #[must_use]
    pub fn success(files: Vec<CompiledFile>) -> Self {
        Self {
            files,
            errors: vec![],
        }
    }

    /// Build an error output with a message.
    #[must_use]
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
pub trait Codegen: Send + Sync {
    /// Generate output files for the given IR program and source files.
    #[must_use]
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput;
}

/// Sanitize a CSS value string to prevent injection attacks.
///
/// Rejects dangerous CSS value patterns that could execute JavaScript:
/// `javascript:`, `expression(`, `-moz-binding`, `data:text/html`.
/// Returns the original string if safe, or an empty string with a warning.
#[must_use]
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
#[must_use]
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
#[must_use]
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
#[must_use]
pub fn compile(
    ir: &IRProgram,
    target: Target,
    #[allow(unused_variables)] source_files: &HashMap<String, String>,
) -> CompileOutput {
    // Validate stdlib imports for the target
    if let Err(errors) = validate_stdlib_imports(ir, target) {
        return CompileOutput {
            files: vec![],
            errors,
        };
    }
    #[allow(unreachable_patterns)]
    match target {
        #[cfg(feature = "codegen-omniview")]
        Target::OmniView => omniview::compile_files(ir, source_files),
        #[cfg(feature = "codegen-python")]
        Target::Python => python::PythonCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-javascript")]
        Target::JavaScript => {
            javascript::JavaScriptCodegen::new(false).generate(ir, source_files)
        }
        #[cfg(feature = "codegen-javascript")]
        Target::TypeScript => {
            javascript::JavaScriptCodegen::new(true).generate(ir, source_files)
        }
        #[cfg(feature = "codegen-rust")]
        Target::Rust => rust::RustCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-dart")]
        Target::Dart => dart::DartCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-react")]
        Target::React => react::ReactCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-vue")]
        Target::Vue => vue::VueCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-dioxus")]
        Target::Dioxus => dioxus::DioxusCodegen.generate(ir, source_files),
        #[cfg(feature = "codegen-flutter")]
        Target::Flutter => flutter::FlutterCodegen.generate(ir, source_files),
        _ => CompileOutput::error(format!(
            "Backend for '{:?}' is not enabled (compile with the corresponding codegen-* feature flag)",
            target
        )),
    }
}

/// Try to format a stdlib function call. Returns `Some(code)` if the call
/// matches a registered stdlib function, `None` otherwise (so backends fall
/// through to their normal CallExpr handling).
#[must_use]
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

// ═══════════════════════════════════════════════════════════
// Shared codegen utilities
// ═══════════════════════════════════════════════════════════

/// Map an OMNI binary operator name to the target language's operator symbol.
/// - `strict_eq`: if true, uses `===`/`!==` (JS-style); otherwise `==`/`!=`
/// - `textual_bool`: if true, uses `and`/`or` (Python-style); otherwise `&&`/`||`
#[must_use]
pub fn binary_op_to_str(op: &str, strict_eq: bool, textual_bool: bool) -> &str {
    match op {
        "add" => "+",
        "sub" => "-",
        "mul" => "*",
        "div" => "/",
        "mod" => "%",
        "eq" => {
            if strict_eq {
                "==="
            } else {
                "=="
            }
        }
        "neq" => {
            if strict_eq {
                "!=="
            } else {
                "!="
            }
        }
        "lt" => "<",
        "gt" => ">",
        "lteq" => "<=",
        "gteq" => ">=",
        "and" => {
            if textual_bool {
                "and"
            } else {
                "&&"
            }
        }
        "or" => {
            if textual_bool {
                "or"
            } else {
                "||"
            }
        }
        "assign" => "=",
        "addassign" => "+=",
        "subassign" => "-=",
        "mulassign" => "*=",
        "divassign" => "/=",
        "modassign" => "%=",
        _ => op,
    }
}

/// Escape a string value for use as a string literal in generated code.
/// Handles backslash, the target quote character, and common whitespace escapes.
#[must_use]
pub fn escape_literal_str(s: &str, quote: char) -> String {
    s.replace('\\', "\\\\")
        .replace(quote, &format!("\\{}", quote))
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('\0', "\\0")
}

/// Format a block IR node as a single expression by taking the last statement's value.
/// Calls `fmt` for expression/return statements, returns `fallback` for empty/non-value blocks.
#[must_use]
pub fn format_block_as_expr<F>(block: &crate::ir::BlockIR, fmt: &F, fallback: &str) -> String
where
    F: Fn(&crate::ir::ExprIR) -> String,
{
    if let Some(last) = block.statements.last() {
        match last {
            crate::ir::StatementIR::ExprStmtIR { expr } => fmt(expr),
            crate::ir::StatementIR::ReturnStmtIR { value } => {
                value.as_ref().map(fmt).unwrap_or_else(|| fallback.into())
            }
            _ => fallback.into(),
        }
    } else {
        fallback.into()
    }
}

/// Capitalize the first character of a string.
#[must_use]
pub fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Convert CamelCase or camelCase to separator-delimited lowercase.
/// "borderRadius" becomes "border-radius" (sep='-') or "border_radius" (sep='_').
/// Handles initial uppercase (PascalCase) without a leading separator.
#[must_use]
pub fn camel_to_separated(s: &str, sep: char) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push(sep);
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a camelCase CSS property name to kebab-case.
/// "backgroundColor" → "background-color", "--customProp" → "--custom-prop"
#[must_use]
pub fn camel_to_kebab(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if c.is_uppercase() {
            result.push('-');
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Extract a CSS value string from an IR expression node.
/// Numbers get "px" suffix; strings are sanitized via sanitize_css_value.
#[must_use]
pub fn format_css_value(value: &crate::ir::ExprIR) -> String {
    match value {
        crate::ir::ExprIR::LiteralExpr { value } => match value {
            serde_json::Value::String(s) => sanitize_css_value(s),
            serde_json::Value::Number(n) => format!("{}px", n),
            _ => format!("{}", value),
        },
        _ => String::new(),
    }
}

/// Generate a CSS style block for component styles.
/// Handles both empty (generates default `.container {}`) and populated style lists.
#[allow(dead_code)]
pub(crate) fn generate_css_style_block(w: &mut CodeWriter, styles: &[crate::ir::StyleRuleIR]) {
    if styles.is_empty() {
        w.emit(".container {");
        w.emit("}");
        return;
    }
    for style in styles {
        let prefix = if style.is_tag_selector { "" } else { "." };
        w.emit(&format!("{}{} {{", prefix, style.class_name));
        for (prop, value) in &style.properties {
            let css_val = format_css_value(value);
            w.emit(&format!("  {}: {};", camel_to_kebab(prop), css_val));
        }
        w.emit("}");
    }
}

// ═══════════════════════════════════════════════════════════
// Shared event data utilities
// ═══════════════════════════════════════════════════════════

/// Emit JS extraction code for a single event data field.
/// Returns (extraction_expression, fallback_value_literal).
/// `pub(crate)` so other codegen targets (Vue, JS) can reuse the DOM-based mapping.
#[must_use]
#[allow(dead_code)]
pub(crate) fn field_extract_js(kind: &EventDataKind, field: &str, raw: &str) -> (String, String) {
    match (kind, field) {
        // PointerEvent
        (EventDataKind::PointerEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::PointerEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::PointerEvent, "button") => (format!("{}.button", raw), "0".into()),
        (EventDataKind::PointerEvent, "pressure") => (format!("{}.pressure", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltX") => (format!("{}.tiltX", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltY") => (format!("{}.tiltY", raw), "0".into()),
        (EventDataKind::PointerEvent, "pointerType") => (format!("{}.pointerType", raw), "\"\"".into()),
        (EventDataKind::PointerEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // TouchEvent
        (EventDataKind::TouchEvent, "touches") => (
            format!("__mergeTouches({})", raw),
            "[]".into()
        ),
        (EventDataKind::TouchEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // WheelEvent
        (EventDataKind::WheelEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::WheelEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaX") => (format!("{}.deltaX", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaY") => (format!("{}.deltaY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaZ") => (format!("{}.deltaZ", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaMode") => (format!("{}.deltaMode", raw), "0".into()),
        (EventDataKind::WheelEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // KeyEvent
        (EventDataKind::KeyEvent, "key") => (format!("{}.key", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, "code") => (format!("{}.code", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ScrollEvent
        (EventDataKind::ScrollEvent, "x") => (format!("{}.target?.scrollLeft", raw), "0".into()),
        (EventDataKind::ScrollEvent, "y") => (format!("{}.target?.scrollTop", raw), "0".into()),

        // InputEvent
        (EventDataKind::InputEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),
        (EventDataKind::InputEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // ToggleEvent
        (EventDataKind::ToggleEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // SubmitEvent — Web
        (EventDataKind::SubmitEvent, "values") => (
            format!("Object.fromEntries(new FormData({}.target))", raw),
            "{}".into()
        ),

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),

        // ClipboardEvent
        (EventDataKind::ClipboardEvent, "text") => (
            format!("(function(){{try{{return {}.clipboardData?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),

        // DragEvent
        (EventDataKind::DragEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DragEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DragEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // DropEvent
        (EventDataKind::DropEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DropEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DropEvent, "files") => (
            format!("[...({}.dataTransfer?.files ?? [])].map(f => f.name)", raw),
            "[]".into()
        ),
        (EventDataKind::DropEvent, "text") => (
            format!("(function(){{try{{return {}.dataTransfer?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),
        (EventDataKind::DropEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ContextMenuEvent
        (EventDataKind::ContextMenuEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // SelectEvent
        (EventDataKind::SelectEvent, "start") => (format!("{}.target?.selectionStart", raw), "0".into()),
        (EventDataKind::SelectEvent, "end") => (format!("{}.target?.selectionEnd", raw), "0".into()),
        (EventDataKind::SelectEvent, "text") => (
            format!("({}.target?.value ?? '').substring({}.target?.selectionStart ?? 0, {}.target?.selectionEnd ?? 0)", raw, raw, raw),
            "\"\"".into()
        ),

        // ResizeEvent
        (EventDataKind::ResizeEvent, "width") => ("window.innerWidth".into(), "0".into()),
        (EventDataKind::ResizeEvent, "height") => ("window.innerHeight".into(), "0".into()),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => (format!("{}.target?.currentTime", raw), "0".into()),
        (EventDataKind::MediaEvent, "duration") => (format!("{}.target?.duration", raw), "0".into()),

        // MediaLoadEvent
        (EventDataKind::MediaLoadEvent, "naturalWidth") => (format!("{}.target?.naturalWidth", raw), "0".into()),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => (format!("{}.target?.naturalHeight", raw), "0".into()),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => (format!("{}.target?.error?.message", raw), "\"unknown\"".into()),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => (format!("{}.animationName", raw), "\"\"".into()),
        (EventDataKind::AnimationEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => (format!("{}.propertyName", raw), "\"\"".into()),
        (EventDataKind::TransitionEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        _ => (format!("undefined /* unknown field {} for {:?} */", field, kind), "undefined".into()),
    }
}

/// Emit a JS object literal for event data extraction from a raw event variable.
/// Only emits fields in `used_fields` (or all fields from schema if None).
/// `pub(crate)` so other codegen targets (Vue, JS) can reuse the DOM-based mapping.
#[allow(dead_code)]
#[must_use]
pub(crate) fn emit_event_object(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
) -> String {
    let kind = match event_data {
        Some(k) => k,
        None => return "{}".to_string(),
    };

    let schema = crate::ir::event_data_schema(kind);
    let fields: Vec<String> = match used_fields {
        Some(fields) if !fields.is_empty() => {
            // Only emit used fields, in schema order
            schema
                .iter()
                .filter(|f| fields.contains(&f.name))
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
        Some(_) => {
            // Empty vec — no fields used
            return "{}".to_string();
        }
        None => {
            // Conservative — emit all fields
            schema
                .iter()
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
    };

    format!("{{ {} }}", fields.join(", "))
}

// ═══════════════════════════════════════════════════════════
// Shared JSON value formatting
// ═══════════════════════════════════════════════════════════

/// Configuration for target-language-specific JSON value formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonFormatConfig {
    pub null_literal: &'static str,
    pub bool_true: &'static str,
    pub bool_false: &'static str,
    pub quote: char,
    pub string_suffix: &'static str,
    pub array_fmt: JsonArrayFmt,
    pub object_fmt: JsonObjectFmt,
    pub strip_trailing_dot_zero: bool,
}

/// How to format JSON arrays for the target language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonArrayFmt {
    /// `[a, b, c]`
    Bracket,
    /// `vec![a, b, c]`
    VecMacro,
    /// Use serde_json Display
    Passthrough,
}

/// How to format JSON objects for the target language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonObjectFmt {
    /// `{ k: v }`
    Bracket,
    /// Use serde_json Display
    Passthrough,
}

/// Format a `serde_json::Value` as a target-language literal.
///
/// Handles null, bool, number, string, array, and object values
/// according to the provided [`JsonFormatConfig`].
#[must_use]
pub fn format_json_value(value: &serde_json::Value, cfg: &JsonFormatConfig) -> String {
    match value {
        serde_json::Value::Null => cfg.null_literal.into(),
        serde_json::Value::Bool(b) => {
            if *b {
                cfg.bool_true.into()
            } else {
                cfg.bool_false.into()
            }
        }
        serde_json::Value::Number(n) => {
            let s = n.to_string();
            if cfg.strip_trailing_dot_zero {
                s.strip_suffix(".0").unwrap_or(&s).to_string()
            } else {
                s
            }
        }
        serde_json::Value::String(s) => {
            let escaped = escape_literal_str(s, cfg.quote);
            if cfg.string_suffix.is_empty() {
                format!("{c}{escaped}{c}", c = cfg.quote)
            } else {
                format!(
                    "{c}{escaped}{c}{suffix}",
                    c = cfg.quote,
                    suffix = cfg.string_suffix
                )
            }
        }
        serde_json::Value::Array(arr) => match cfg.array_fmt {
            JsonArrayFmt::Bracket => {
                let items: Vec<_> = arr.iter().map(|v| format_json_value(v, cfg)).collect();
                format!("[{}]", items.join(", "))
            }
            JsonArrayFmt::VecMacro => {
                let items: Vec<_> = arr.iter().map(|v| format_json_value(v, cfg)).collect();
                format!("vec![{}]", items.join(", "))
            }
            JsonArrayFmt::Passthrough => value.to_string(),
        },
        serde_json::Value::Object(obj) => match cfg.object_fmt {
            JsonObjectFmt::Bracket => {
                let items: Vec<_> = obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, format_json_value(v, cfg)))
                    .collect();
                format!("{{ {} }}", items.join(", "))
            }
            JsonObjectFmt::Passthrough => value.to_string(),
        },
    }
}

// ── Config constants per backend family ─────────────────

/// Config: JavaScript / TypeScript
pub const JSON_JS: JsonFormatConfig = JsonFormatConfig {
    null_literal: "null",
    bool_true: "true",
    bool_false: "false",
    quote: '"',
    string_suffix: "",
    array_fmt: JsonArrayFmt::Bracket,
    object_fmt: JsonObjectFmt::Bracket,
    strip_trailing_dot_zero: false,
};

/// Config: React (same as JS)
pub const JSON_REACT: JsonFormatConfig = JSON_JS;

/// Config: Vue (same as JS)
pub const JSON_VUE: JsonFormatConfig = JSON_JS;

/// Config: OmniView (same as JS)
pub const JSON_OMNIVIEW: JsonFormatConfig = JSON_JS;

/// Config: Dart
pub const JSON_DART: JsonFormatConfig = JsonFormatConfig {
    null_literal: "null",
    bool_true: "true",
    bool_false: "false",
    quote: '\'',
    string_suffix: "",
    array_fmt: JsonArrayFmt::Bracket,
    object_fmt: JsonObjectFmt::Bracket,
    strip_trailing_dot_zero: true,
};

/// Config: Flutter (same as Dart)
pub const JSON_FLUTTER: JsonFormatConfig = JSON_DART;

/// Config: Rust
pub const JSON_RUST: JsonFormatConfig = JsonFormatConfig {
    null_literal: "None",
    bool_true: "true",
    bool_false: "false",
    quote: '"',
    string_suffix: ".to_string()",
    array_fmt: JsonArrayFmt::VecMacro,
    object_fmt: JsonObjectFmt::Passthrough,
    strip_trailing_dot_zero: true,
};

/// Config: Dioxus (same as Rust)
pub const JSON_DIOXUS: JsonFormatConfig = JSON_RUST;

/// Config: Python
pub const JSON_PYTHON: JsonFormatConfig = JsonFormatConfig {
    null_literal: "None",
    bool_true: "True",
    bool_false: "False",
    quote: '\'',
    string_suffix: "",
    array_fmt: JsonArrayFmt::Passthrough,
    object_fmt: JsonObjectFmt::Passthrough,
    strip_trailing_dot_zero: true,
};

#[cfg(test)]
mod tests_property;

#[cfg(all(test, feature = "all-backends"))]
mod tests_regression;

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

    #[test]
    fn test_emit_fmt_basic() {
        let mut w = CodeWriter::new();
        w.emit_fmt(format_args!("let x = {};", 42));
        assert_eq!(w.into_string(), "let x = 42;\n");
    }

    #[test]
    fn test_emit_fmt_with_indent() {
        let mut w = CodeWriter::new();
        w.emit("fn main() {");
        w.indent();
        w.emit_fmt(format_args!("let x = {};", 1));
        w.dedent();
        w.emit("}");
        assert_eq!(w.into_string(), "fn main() {\n    let x = 1;\n}\n");
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
