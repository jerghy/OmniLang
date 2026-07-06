//! OmniLang CLI — `omnic` binary.
//!
//! Exact Rust port of the Python `omnilang/cli.py` reference implementation.
//!
//! Commands:
//!   build    — compile .omni source to target language(s)
//!   analyse  — static analysis on .omni source
//!   run      — compile and run an OmniLang file
//!   create   — scaffold a new OmniLang project
//!   watch    — watch .omni files and auto-rebuild on changes

mod api;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

// Mirrors Python ALL_TARGETS in cli.py:27
const ALL_TARGETS: &[&str] = &[
    "python", "js", "ts", "rust", "dart", "react", "vue", "dioxus", "flutter", "omniview",
];

#[derive(Parser)]
#[command(name = "omnic")]
#[command(version = "0.2.0")]
#[command(
    about = "OmniLang compiler — transpile .omni to Python, JS, TS, Rust, Dart, React, Vue, and more"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile .omni source file(s) to target language(s)
    Build {
        /// Source file(s) (.omni) or directory (default: auto-discover src/)
        files: Vec<String>,
        /// Target language/framework
        #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(ALL_TARGETS))]
        target: Option<String>,
        /// Output directory (default: ./build)
        #[arg(short, long, default_value = "./build")]
        output: String,
    },

    /// Run static analysis on .omni source
    Analyse {
        /// Source file(s) (.omni) or directory (default: auto-discover src/)
        files: Vec<String>,
        /// Output format
        #[arg(long, default_value = "terminal", value_parser = clap::builder::PossibleValuesParser::new(["terminal", "json"]))]
        format: String,
    },

    /// Compile and run an OmniLang file
    Run {
        /// Source file (.omni) — defaults to src/main.omni when in project root
        file: Option<String>,
        /// Target (auto-detected if omitted)
        #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(ALL_TARGETS))]
        target: Option<String>,
    },

    /// Scaffold a new OmniLang project
    Create {
        /// Project name
        name: String,
        /// Project type (required): logic or view
        #[arg(short, long, required = true, value_parser = clap::builder::PossibleValuesParser::new(["logic", "view"]))]
        r#type: String,
    },

    /// Watch .omni files and auto-rebuild on changes
    Watch {
        /// Source file (.omni)
        file: String,
        /// Target (default: react)
        #[arg(short, long, default_value = "react", value_parser = clap::builder::PossibleValuesParser::new(ALL_TARGETS))]
        target: String,
        /// Output directory (default: ./build)
        #[arg(short, long, default_value = "./build")]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Analyse { files, format } => cmd_analyse(files, &format),
        Command::Build {
            files,
            target,
            output,
        } => cmd_build(files, target, &output),
        Command::Run { file, target } => cmd_run(file, target),
        Command::Create { name, r#type } => cmd_create(&name, &r#type),
        Command::Watch {
            file,
            target,
            output,
        } => cmd_watch(&file, &target, &output),
    };

    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

// ═══════════════════════════════════════════════════════════
// Helpers — mirrors cli.py helpers
// ═══════════════════════════════════════════════════════════
// Analyse — mirrors Python cmd_analyse (cli.py:194)
// ═══════════════════════════════════════════════════════════

/// Validate an output filename — rejects characters that could cause
/// path traversal or filesystem issues beyond standard OS protections.
/// Note: `sanitize_module_name` in `codegen/mod.rs` is a sanitizer (replaces bad chars);
/// this function is a validator (rejects outright). They serve different purposes.
fn validate_output_filename(name: &str) -> anyhow::Result<()> {
    if name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '/' || c == '-')
    {
        Ok(())
    } else {
        anyhow::bail!("Invalid character in output filename: {name}")
    }
}

fn cmd_analyse(files: Vec<String>, format: &str) -> anyhow::Result<()> {
    let sources = api::collect_sources(&files)?;
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

    // Build the same request JSON that Python sends to RustBridge.analyse()
    // This calls the FFI omni_analyse which does 3-phase analysis:
    //   Phase 1: Parse all sources
    //   Phase 2: Semantic analysis + IR lowering
    //   Phase 3: Lint rules
    let request = serde_json::json!({
        "sources": sources,
        "path_mapping": path_mapping,
    });

    // Use the same pipeline as the FFI omni_analyse — directly, not via DLL
    let result = api::analyse_direct(request);

    if format == "json" {
        // Compact JSON — matches Python json.dumps(...) without indent
        let json = serde_json::to_string(&result)?;
        println!("{json}");
        if !result["success"].as_bool().unwrap_or(false) {
            std::process::exit(1);
        }
        return Ok(());
    }

    // Terminal output — matches Python OmniError.format() (errors.py:19)
    if let Some(diags) = result["diagnostics"].as_array() {
        let error_count = diags.iter().filter(|d| d["severity"] == "error").count();
        let warn_count = diags.iter().filter(|d| d["severity"] == "warning").count();

        for d in diags {
            let severity = d["severity"].as_str().unwrap_or("info");
            let code = d["code"].as_str().unwrap_or("");
            let message = d["message"].as_str().unwrap_or("");
            let file = d["file"].as_str();
            let line = d["line"].as_u64();
            let col = d["col"].as_u64();
            let suggestion = d["suggestion"].as_str();

            // Python format: "Severity: message\n  -> file:line:col (CODE)"
            let sev = match severity {
                "error" => "Error",
                "warning" => "Warning",
                _ => "Info",
            };
            let mut line_str = format!("{sev}: {message}");
            let mut loc = String::new();
            if let Some(f) = file {
                loc.push_str(f);
                if let Some(l) = line {
                    loc.push_str(&format!(":{l}"));
                    if let Some(c) = col {
                        loc.push_str(&format!(":{c}"));
                    }
                }
            }
            if !loc.is_empty() {
                line_str.push_str(&format!("\n  -> {loc} ({code})"));
            }
            if let Some(sugg) = suggestion {
                if !sugg.is_empty() {
                    line_str.push_str(&format!("\n  Suggestion: {sugg}"));
                }
            }
            println!("{line_str}");
        }

        // Summary — matches Python line 236-243
        if error_count > 0 {
            println!("\n{} diagnostic(s) — {error_count} error(s)", diags.len());
            std::process::exit(1);
        }
        if warn_count > 0 {
            println!("\n{warn_count} warning(s)");
            return Ok(());
        }
    }

    println!("No issues found.");
    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Build — mirrors Python cmd_build (cli.py:144)
// ═══════════════════════════════════════════════════════════

fn cmd_build(files: Vec<String>, target: Option<String>, output_dir: &str) -> anyhow::Result<()> {
    let sources = api::collect_sources(&files)?;
    let target_str = api::detect_target(&sources, target.as_deref());
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

    let t: omnilang::codegen::Target = target_str
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown target: {target_str}"))?;

    let result_json = omnilang::compile_multi_to_target(sources, t, path_mapping, false);
    let data: serde_json::Value = serde_json::from_str(&result_json)?;

    // Python checks result.success (cli.py:173)
    if !data["success"].as_bool().unwrap_or(false) {
        let msg = extract_error_messages(&data["errors"]);
        anyhow::bail!("{msg}");
    }

    // Write output files — mirrors Python cli.py:179-187
    let output_path = Path::new(output_dir);
    std::fs::create_dir_all(output_path)?;

    if let Some(files) = data["files"].as_object() {
        for (filename, content) in files {
            validate_output_filename(filename)?;
            let content = match content.as_str() {
                Some(s) => s,
                None => {
                    eprintln!(
                        "Warning: null or non-string content for file '{}', skipping",
                        filename
                    );
                    continue;
                }
            };
            let out_path = output_path.join(filename);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&out_path, content)?;
            println!("  -> {}", out_path.display());
        }
        println!(
            "Compiled {} file(s) to {target_str} successfully.",
            files.len()
        );
    }

    Ok(())
}

/// Extract human-readable error messages from JSON errors array.
/// Supports both structured Diagnostic objects ({message, ...}) and legacy plain-string errors.
fn extract_error_messages(errors: &serde_json::Value) -> String {
    errors
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| {
                    v.get("message")
                        .and_then(|m| m.as_str())
                        .or_else(|| v.as_str())
                        .unwrap_or("?")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "Compilation failed.".to_string())
}

// ═══════════════════════════════════════════════════════════
// Run — mirrors Python cmd_run (cli.py:280)
// ═══════════════════════════════════════════════════════════

fn cmd_run(file: Option<String>, target: Option<String>) -> anyhow::Result<()> {
    // Auto-detect target — mirrors Python cli.py:312-316
    let run_target = match target {
        Some(t) => t,
        None => {
            let sources = api::collect_sources(&[])?;
            let detected = api::detect_target(&sources, None);
            match detected.as_str() {
                "react" | "vue" | "dioxus" | "flutter" => "omniview".to_string(),
                t => t.to_string(),
            }
        }
    };

    let (output_dir, first_name) = compile_to_temp(&file, &run_target)?;

    // OmniView: assemble standalone HTML — mirrors Python cli.py:339-350
    if run_target == "omniview" {
        return assemble_omniview_html(&output_dir);
    }

    // Execute based on target — mirrors Python cli.py:361-381
    let out_path = output_dir.join(&first_name);
    match run_target.as_str() {
        "python" => run_python(&out_path),
        "js" => run_node(&out_path),
        "ts" => {
            let status = std::process::Command::new("npx")
                .args(["ts-node", &out_path.to_string_lossy()])
                .status()?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
            Ok(())
        }
        "rust" => run_rust(&out_path),
        "dart" => run_dart(&out_path),
        other => {
            eprintln!("Don't know how to run target: {other}");
            std::process::exit(1);
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Helpers — sub-functions extracted from cmd_run
// ═══════════════════════════════════════════════════════════

/// Compile OmniLang source to a temporary build directory and return (output_dir, first_output_filename).
fn compile_to_temp(file: &Option<String>, run_target: &str) -> anyhow::Result<(PathBuf, String)> {
    // Auto-discover src/main.omni when no file — mirrors Python cli.py:293-302
    let filepath = match file {
        Some(f) => f.clone(),
        None => {
            let default = Path::new("src/main.omni");
            if default.exists() {
                default.to_string_lossy().to_string()
            } else {
                anyhow::bail!(
                    "No source file specified and src/main.omni not found. \
                     Run 'omnic create <name>' to create a project, or specify a file."
                );
            }
        }
    };

    let path = Path::new(&filepath);
    if !path.exists() {
        anyhow::bail!("File not found: {filepath}");
    }

    // Collect ALL project sources — mirrors Python cli.py:309
    let sources = api::collect_sources(&[])?;

    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);
    let t: omnilang::codegen::Target = run_target
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown target: {run_target}"))?;

    let result_json = omnilang::compile_multi_to_target(sources, t, path_mapping, false);
    let data: serde_json::Value = serde_json::from_str(&result_json)?;

    if !data["success"].as_bool().unwrap_or(false) {
        let msg = extract_error_messages(&data["errors"]);
        anyhow::bail!("{msg}");
    }

    // Output to build/debug/ — mirrors Python cli.py:327-336
    let output_dir = Path::new("build/debug");
    std::fs::create_dir_all(output_dir)?;

    if let Some(files) = data["files"].as_object() {
        for (filename, content) in files {
            validate_output_filename(filename)?;
            let content = match content.as_str() {
                Some(s) => s,
                None => {
                    eprintln!(
                        "Warning: null or non-string content for file '{}', skipping",
                        filename
                    );
                    continue;
                }
            };
            let out_path = output_dir.join(filename);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&out_path, content)?;
        }
    }

    // Locate first output file — mirrors Python cli.py:352-357
    let first_name = data["files"]
        .as_object()
        .and_then(|m| m.keys().next())
        .cloned()
        .unwrap_or_default();
    if first_name.is_empty() {
        eprintln!("No output generated.");
        std::process::exit(1);
    }

    Ok((output_dir.to_path_buf(), first_name))
}

/// Assemble an OmniView standalone HTML file from compiled output files.
fn assemble_omniview_html(output_dir: &Path) -> anyhow::Result<()> {
    let read_file = |name: &str| -> String {
        let path = output_dir.join(name);
        std::fs::read_to_string(path).unwrap_or_default()
    };

    // Check both body.html and index.html — Python cli.py:342
    let body = {
        let b = read_file("body.html");
        if b.is_empty() {
            read_file("index.html")
        } else {
            b
        }
    };
    let mut js = read_file("index.js");
    let css = read_file("index.css");

    // JS workaround for animation init — Python cli.py:255-260
    js.push_str(
        "\nrequestAnimationFrame(function(){ requestAnimationFrame(function(){
  var el = document.querySelector('.app');
  if (el && el.style.opacity === '0') { el.style.removeProperty('opacity'); el.style.removeProperty('transform'); }
}); });\n",
    );

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OmniView App</title>
    <style>{css}</style>
</head>
<body>
    <div id="root">{body}</div>
    <script>{js}</script>
</body>
</html>"#
    );

    let index_path = output_dir.join("index.html");
    std::fs::write(&index_path, &html)?;

    let abs_path = std::fs::canonicalize(&index_path)?;
    println!("Open {} in your browser", abs_path.display());
    Ok(())
}

/// Execute a Python file with `python <path>`.
fn run_python(path: &Path) -> anyhow::Result<()> {
    let status = std::process::Command::new("python").arg(path).status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

/// Execute a JavaScript file with `node <path>`.
fn run_node(path: &Path) -> anyhow::Result<()> {
    let status = std::process::Command::new("node").arg(path).status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

/// Execute a Rust program by copying to a Cargo project and running `cargo run`.
fn run_rust(path: &Path) -> anyhow::Result<()> {
    let rust_dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("run_rust: path has no parent: {:?}", path))?
        .join("rust_proj");
    let src_dir = rust_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;
    std::fs::copy(path, src_dir.join("main.rs"))?;
    std::fs::write(
        rust_dir.join("Cargo.toml"),
        r#"[package]
name = "omni_run"
version = "0.1.0"
edition = "2021"
"#,
    )?;
    let status = std::process::Command::new("cargo")
        .args(["run", "--quiet"])
        .current_dir(&rust_dir)
        .status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

/// Execute a Dart file with `dart run <path>`.
fn run_dart(path: &Path) -> anyhow::Result<()> {
    let status = std::process::Command::new("dart")
        .args(["run", &path.to_string_lossy()])
        .status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
fn cmd_create(name: &str, project_type: &str) -> anyhow::Result<()> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("Directory '{name}' already exists.");
    }

    std::fs::create_dir_all(project_dir)?;

    // .omnilang.toml — mirrors Python cli.py:406-417
    let config = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"

[source]
dir = "src"

[output]
dir = "build"
"#
    );
    std::fs::write(project_dir.join(".omnilang.toml"), &config)?;
    println!("  Config: {}/.omnilang.toml", project_dir.display());

    // src/
    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // main.omni — mirrors Python LOGIC_TEMPLATE / VIEW_TEMPLATE
    let template = if project_type == "logic" {
        format!(
            "// {name} — OmniLang logic project\n\
             fn main() {{\n    print(\"Hello from {name}!\");\n}}\n"
        )
    } else {
        format!(
            "// {name} — OmniLang view project\n\
             import omniview;\n\n\
             component App {{\n\n\
             \n    view {{\n\n\
             \n        <div class: app>\n\
             \n            <h1>{name}</h1>\n\
             \n            <p>Welcome to your OmniView app!</p>\n\
             \n        </div>\n\
             \n    }}\n\
             \n}}\n"
        )
    };
    std::fs::write(src_dir.join("main.omni"), &template)?;
    println!("  Source: {}/src/main.omni", project_dir.display());

    println!(
        "\nCreated {project_type} project '{name}' at {}",
        std::fs::canonicalize(project_dir)?.display()
    );
    println!("  cd {name}");
    println!("  omnic run src/main.omni");
    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Watch — mirrors Python cmd_watch (cli.py:477)
// ═══════════════════════════════════════════════════════════

fn cmd_watch(file: &str, target: &str, output_dir: &str) -> anyhow::Result<()> {
    let filepath = Path::new(file);
    if !filepath.exists() {
        anyhow::bail!("File not found: {file}");
    }

    println!("Watching {file} (target={target}, output={output_dir})");
    println!("Press Ctrl+C to stop.\n");

    let mut last_mtime = std::fs::metadata(filepath)
        .ok()
        .and_then(|m| m.modified().ok());

    let mut build_count: u64 = 0;

    let t: omnilang::codegen::Target = target
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown target: {target}"))?;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        let current = std::fs::metadata(filepath)
            .ok()
            .and_then(|m| m.modified().ok());

        if current == last_mtime {
            continue;
        }
        last_mtime = current;
        build_count += 1;

        let timestamp = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs() % 86400;
            format!(
                "{:02}:{:02}:{:02}",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            )
        };

        // Read source — Python cli.py:517
        let source = match std::fs::read_to_string(filepath) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[{timestamp}] #{build_count} Read error: {e}");
                continue;
            }
        };

        let mut sources = HashMap::new();
        sources.insert(file.to_string(), source);
        let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

        let result_json = omnilang::compile_multi_to_target(sources, t, path_mapping, false);
        let data: serde_json::Value = match serde_json::from_str(&result_json) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[{timestamp}] #{build_count} JSON error: {e}");
                continue;
            }
        };

        if data["success"].as_bool().unwrap_or(false) {
            // Write files — Python cli.py:526-531
            std::fs::create_dir_all(output_dir)?;
            if let Some(files) = data["files"].as_object() {
                for (fname, content) in files {
                    if let Err(e) = validate_output_filename(fname) {
                        eprintln!("[{timestamp}] #{build_count} Invalid filename: {e}");
                        continue;
                    }
                    let out_path = Path::new(output_dir).join(fname);
                    if let Some(parent) = out_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!("[{timestamp}] #{build_count} Create dir error: {e}");
                            continue;
                        }
                    }
                    let content_str = match content.as_str() {
                        Some(s) => s,
                        None => {
                            eprintln!("[{timestamp}] #{build_count} Warning: null or non-string content for file '{fname}', skipping");
                            continue;
                        }
                    };
                    if let Err(e) = std::fs::write(&out_path, content_str) {
                        eprintln!("[{timestamp}] #{build_count} Write error: {e}");
                        continue;
                    }
                }
                let file_count = files.len();
                println!(
                    "[{timestamp}] #{build_count} Build succeeded — {file_count} file(s) -> {output_dir}/"
                );
            }
        } else {
            // Error output — Python cli.py:535-541
            let msg = data["errors"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|v| v.as_str().unwrap_or("?"))
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_else(|| "Build failed".to_string());
            println!("[{timestamp}] #{build_count} {msg}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_output_filename_valid() {
        assert!(validate_output_filename("index.js").is_ok());
        assert!(validate_output_filename("src/main.rs").is_ok());
        assert!(validate_output_filename("my_file.test.ts").is_ok());
        assert!(validate_output_filename("a-b/c.js").is_ok());
        assert!(validate_output_filename("index.html").is_ok());
        assert!(validate_output_filename("style.css").is_ok());
    }

    #[test]
    fn validate_output_filename_invalid() {
        assert!(validate_output_filename("file with spaces.js").is_err());
        assert!(validate_output_filename("file|bad.ts").is_err());
        assert!(validate_output_filename("file<bad>.ts").is_err());
        assert!(validate_output_filename("file?bad.ts").is_err());
        assert!(validate_output_filename("file*bad.ts").is_err());
        assert!(validate_output_filename("\"quote\".ts").is_err());
    }

    #[test]
    fn extract_error_messages_structured() {
        let json = serde_json::json!([
            {"message": "parse error at line 3", "code": "E0001"},
            {"message": "unresolved variable 'x'", "code": "E0002"},
        ]);
        let msg = extract_error_messages(&json);
        assert!(
            msg.contains("parse error at line 3"),
            "should contain structured error message, got: {msg}"
        );
        assert!(
            msg.contains("unresolved variable 'x'"),
            "should contain second error message, got: {msg}"
        );
        let lines: Vec<&str> = msg.split('\n').collect();
        assert_eq!(lines.len(), 2, "should have two lines, got: {msg:?}");
    }

    #[test]
    fn extract_error_messages_legacy_strings() {
        let json = serde_json::json!(["compilation failed", "missing semicolon"]);
        let msg = extract_error_messages(&json);
        assert!(msg.contains("compilation failed"), "got: {msg}");
        assert!(msg.contains("missing semicolon"), "got: {msg}");
    }

    #[test]
    fn extract_error_messages_fallback() {
        let json = serde_json::json!([]);
        let msg = extract_error_messages(&json);
        assert_eq!(msg, "", "empty array should produce empty string");

        let json = serde_json::json!({"not": "an array"});
        let msg = extract_error_messages(&json);
        assert_eq!(msg, "Compilation failed.", "non-array should fall back");
    }

    #[test]
    fn derive_path_mapping_basic() {
        let mut sources = std::collections::HashMap::new();
        sources.insert("src/main.omni".to_string(), "fn main() {}".to_string());
        sources.insert(
            "src/lib/helper.omni".to_string(),
            "pub fn help() {}".to_string(),
        );
        let mapping = omnilang::path_mapping::derive_path_mapping(&sources);

        assert_eq!(
            mapping.get("src/main.omni"),
            Some(&"src/main.omni".to_string())
        );
        assert_eq!(
            mapping.get("./lib/helper.omni"),
            Some(&"src/lib/helper.omni".to_string())
        );
        assert_eq!(
            mapping.get("helper.omni"),
            Some(&"src/lib/helper.omni".to_string())
        );
    }
}
