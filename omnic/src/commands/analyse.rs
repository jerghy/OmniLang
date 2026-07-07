//! `omnic analyse` — static analysis on .omni source.
//! Mirrors Python cmd_analyse (cli.py:194).

use crate::api;

pub fn cmd_analyse(files: Vec<String>, format: &str) -> anyhow::Result<()> {
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
            anyhow::bail!("analysis failed");
        }
        return Ok(());
    }

    // Terminal output — matches Python OmniError.format() (errors.py:19)
    if let Some(diags) = result["diagnostics"].as_array() {
        let error_count = diags.iter().filter(|d| d["severity"] == "error").count();
        let warn_count = diags
            .iter()
            .filter(|d| d["severity"] == "warning")
            .count();

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
            anyhow::bail!("analyse found {error_count} error(s)");
        }
        if warn_count > 0 {
            println!("\n{warn_count} warning(s)");
            return Ok(());
        }
    }

    println!("No issues found.");
    Ok(())
}
