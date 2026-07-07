//! Shared helper functions used by multiple CLI commands.

use std::path::Path;

/// Validate an output filename — rejects characters that could cause
/// path traversal or filesystem issues beyond standard OS protections.
/// Note: `sanitize_module_name` in `codegen/mod.rs` is a sanitizer (replaces bad chars);
/// this function is a validator (rejects outright). They serve different purposes.
pub fn validate_output_filename(name: &str) -> anyhow::Result<()> {
    // Reject empty filenames
    if name.is_empty() {
        anyhow::bail!("Output filename must not be empty");
    }
    // Reject null bytes (path injection)
    if name.contains('\0') {
        anyhow::bail!("Output filename must not contain null bytes: {name:?}");
    }
    // Reject absolute paths (starts with '/' or '\')
    if name.starts_with('/') || name.starts_with('\\') {
        anyhow::bail!("Output filename must not be absolute: {name}");
    }
    // Reject parent directory traversal
    if name.contains("..") {
        anyhow::bail!("Output filename must not contain '..': {name}");
    }
    if name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '/' || c == '-')
    {
        Ok(())
    } else {
        anyhow::bail!("Invalid character in output filename: {name}")
    }
}

/// Extract human-readable error messages from JSON errors array.
/// Supports both structured Diagnostic objects ({message, ...}) and legacy plain-string errors.
pub fn extract_error_messages(errors: &serde_json::Value) -> String {
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

/// Write compiled output files to disk.
/// Validates filenames, creates parent directories, and writes content.
/// Returns the number of files successfully written.
pub fn write_output_files(
    output_dir: &Path,
    files_opt: Option<&serde_json::Map<String, serde_json::Value>>,
) -> anyhow::Result<usize> {
    let Some(files) = files_opt else {
        return Ok(0);
    };
    let mut count = 0;
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
        count += 1;
    }
    Ok(count)
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
    fn validate_output_filename_empty() {
        assert!(validate_output_filename("").is_err());
    }

    #[test]
    fn validate_output_filename_null_byte() {
        assert!(validate_output_filename("bad\0file.js").is_err());
        assert!(validate_output_filename("sub\0dir/file.js").is_err());
    }

    #[test]
    fn validate_output_filename_absolute_unix() {
        assert!(validate_output_filename("/etc/passwd").is_err());
        assert!(validate_output_filename("/tmp/output.js").is_err());
    }

    #[test]
    fn validate_output_filename_absolute_windows() {
        assert!(validate_output_filename("\\windows\\system32").is_err());
        assert!(validate_output_filename("\\absolute\\path.js").is_err());
    }

    #[test]
    fn validate_output_filename_traversal_valid() {
        // Relative paths with subdirectories are fine
        assert!(validate_output_filename("src/output.js").is_ok());
        assert!(validate_output_filename("build/js/app.js").is_ok());
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
        let mut sources = rustc_hash::FxHashMap::default();
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
