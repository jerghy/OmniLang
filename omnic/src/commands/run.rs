//! `omnic run` — compile and run an OmniLang file.
//! Mirrors Python cmd_run (cli.py:280).

use std::path::{Path, PathBuf};

use crate::api;
use crate::commands::common::{extract_error_messages, print_warnings, write_output_files};

pub fn cmd_run(file: Option<String>, target: Option<String>) -> anyhow::Result<()> {
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
                anyhow::bail!("ts-node exited with code {:?}", status.code());
            }
            Ok(())
        }
        "rust" => run_rust(&out_path),
        "dart" => run_dart(&out_path),
        other => {
            anyhow::bail!("Don't know how to run target: {other}");
        }
    }
}

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

    let result_json =
        omnilang::compile_multi_to_target(sources, t, path_mapping, false, false, None);
    let data: serde_json::Value = serde_json::from_str(&result_json)?;

    if !data["success"].as_bool().unwrap_or(false) {
        let msg = extract_error_messages(&data["errors"]);
        anyhow::bail!("{msg}");
    }

    // Surface warnings (lint rules + codegen) on stderr before running.
    print_warnings(&data["warnings"]);

    // Output to build/debug/ — mirrors Python cli.py:327-336
    let output_dir = Path::new("build/debug");
    std::fs::create_dir_all(output_dir)?;

    write_output_files(output_dir, data["files"].as_object())?;

    // Locate first output file — mirrors Python cli.py:352-357
    let first_name = data["files"]
        .as_object()
        .and_then(|m| m.keys().next())
        .cloned()
        .unwrap_or_default();
    if first_name.is_empty() {
        anyhow::bail!("No output generated.");
    }

    Ok((output_dir.to_path_buf(), first_name))
}

/// Assemble a standalone OmniView HTML file using the shared utility.
fn assemble_omniview_html(output_dir: &Path) -> anyhow::Result<()> {
    use omnilang::codegen::omniview::wrap_debug_html;

    let body_html = read_file_opt(output_dir, "body.html")
        .or_else(|| read_file_opt(output_dir, "index.html"))
        .unwrap_or_default();
    let js = read_file_opt(output_dir, "index.js").unwrap_or_default();
    let css = read_file_opt(output_dir, "index.css").unwrap_or_default();

    let html = wrap_debug_html(&js, &css, &body_html);

    let index_path = output_dir.join("index.html");
    std::fs::write(&index_path, &html)?;

    let abs_path = std::fs::canonicalize(&index_path)?;
    println!("Open {} in your browser", abs_path.display());
    Ok(())
}

fn read_file_opt(output_dir: &Path, name: &str) -> Option<String> {
    let path = output_dir.join(name);
    match std::fs::read_to_string(&path) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            eprintln!("warning: failed to read {}: {}", path.display(), e);
            None
        }
    }
}

/// Execute a Python file with `python <path>`.
fn run_python(path: &Path) -> anyhow::Result<()> {
    let mut child = std::process::Command::new("python").arg(path).spawn()?;
    wait_with_timeout(&mut child, std::time::Duration::from_secs(30))
}

/// Execute a JavaScript file with `node <path>`.
fn run_node(path: &Path) -> anyhow::Result<()> {
    let mut child = std::process::Command::new("node").arg(path).spawn()?;
    wait_with_timeout(&mut child, std::time::Duration::from_secs(30))
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
    let mut child = std::process::Command::new("cargo")
        .args(["run", "--quiet"])
        .current_dir(&rust_dir)
        .spawn()?;
    wait_with_timeout(&mut child, std::time::Duration::from_secs(120))
}

/// Execute a Dart file with `dart run <path>`.
fn run_dart(path: &Path) -> anyhow::Result<()> {
    let mut child = std::process::Command::new("dart")
        .args(["run", &path.to_string_lossy()])
        .spawn()?;
    wait_with_timeout(&mut child, std::time::Duration::from_secs(30))
}

/// Wait for a child process with a timeout, polling with try_wait.
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(status) => {
                if status.success() {
                    return Ok(());
                }
                anyhow::bail!("Process exited with: {status}");
            }
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill(); // process may have exited between try_wait() and kill()
                    anyhow::bail!("Process timed out after {}s", timeout.as_secs());
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}
