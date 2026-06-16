//! Codegen module -- multi-target code generation from Intent IR.
//!
//! Currently supported targets:
//! - OmniView (mature): JS/HTML/CSS with inline signal runtime
//! - Python, JavaScript, Rust, Dart (planned): pure-logic codegen
//! - React, Vue, Dioxus, Flutter (planned): framework-based codegen

pub mod omniview;
pub mod python;
pub mod javascript;
pub mod rust;
pub mod dart;
pub mod react;
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

/// Compile IR to the specified target.
///
/// Routes to each target's codegen module. Framework targets and
/// stub modules return error until implemented.
pub fn compile(
    ir: &IRProgram,
    target: Target,
    source_files: &HashMap<String, String>,
) -> CompileOutput {
    match target {
        Target::OmniView => omniview::compile_files(ir, source_files),
        Target::Python => python::PythonCodegen.generate(ir, source_files),
        Target::JavaScript => javascript::JavaScriptCodegen::new(false).generate(ir, source_files),
        Target::TypeScript => javascript::JavaScriptCodegen::new(true).generate(ir, source_files),
        Target::Rust => rust::RustCodegen.generate(ir, source_files),
        Target::Dart => dart::DartCodegen.generate(ir, source_files),
        Target::React => react::ReactCodegen.generate(ir, source_files),
        Target::Vue => vue::VueCodegen.generate(ir, source_files),
        Target::Dioxus => {
            CompileOutput::error("Dioxus codegen not yet implemented in Rust".into())
        }
        Target::Flutter => {
            CompileOutput::error("Flutter codegen not yet implemented in Rust".into())
        }
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
}
