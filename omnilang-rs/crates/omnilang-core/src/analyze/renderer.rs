use crate::analyze::diagnostic::{Diagnostic, DiagnosticSummary};
use colored::*;

/// Render diagnostics to terminal (rustc-style output).
pub fn render_terminal(diags: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diags {
        let sev = match d.severity {
            super::diagnostic::Severity::Error => "Error".red().bold(),
            super::diagnostic::Severity::Warning => "Warning".yellow().bold(),
            super::diagnostic::Severity::Port => "Port".cyan().bold(),
            super::diagnostic::Severity::Style => "Style".blue().bold(),
            super::diagnostic::Severity::Perf => "Perf".magenta().bold(),
            super::diagnostic::Severity::Sec => "Security".red().bold(),
            super::diagnostic::Severity::Hint => "Hint".dimmed(),
        };
        out.push_str(&format!("{} [{}]: {}\n", sev, d.code, d.message));
        if let Some(ref loc) = d.location {
            out.push_str(&format!("  --> {}:{}:{}\n", loc.file, loc.line, loc.column));
        }
        if let Some(ref help) = d.help {
            out.push_str(&format!("  = help: {}\n", help));
        }
        out.push('\n');
    }
    out
}

/// Render diagnostics as JSON.
pub fn render_json(diags: &[Diagnostic]) -> String {
    let summary = DiagnosticSummary::from_diagnostics(diags);
    let obj = serde_json::json!({
        "diagnostics": diags,
        "summary": {
            "errors": summary.errors,
            "warnings": summary.warnings,
            "portability": summary.ports,
            "style": summary.styles,
            "hints": summary.hints,
        }
    });
    serde_json::to_string_pretty(&obj).unwrap_or_default()
}

/// Render diagnostics in GitHub Actions format.
pub fn render_github(diags: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diags {
        let level = match d.severity {
            super::diagnostic::Severity::Error => "error",
            _ => "warning",
        };
        if let Some(ref loc) = d.location {
            out.push_str(&format!(
                "::{} file={},line={},col={}::[{}] {}\n",
                level, loc.file, loc.line, loc.column, d.code, d.message
            ));
        }
    }
    out
}

/// Render a one-line summary.
pub fn render_summary(diags: &[Diagnostic]) -> String {
    let s = DiagnosticSummary::from_diagnostics(diags);
    let mut parts = vec![];
    if s.errors > 0 { parts.push(format!("{} error(s)", s.errors)); }
    if s.warnings > 0 { parts.push(format!("{} warning(s)", s.warnings)); }
    if s.ports > 0 { parts.push(format!("{} portability issue(s)", s.ports)); }
    if parts.is_empty() { "No issues found.".into() } else { parts.join(", ") }
}
