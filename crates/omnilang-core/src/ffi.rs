//! Native C FFI (for Python ctypes/cffi).
//!
//! Functions: omni_free_string, omni_version, omni_analyse.
//!
//! Note: omni_compile_multi_to_target moved to the omnilang facade crate.

use rustc_hash::FxHashMap;
use std::ffi::CString;
use std::os::raw::c_char;

use serde::Deserialize;

/// Convert a diagnostic to the unified FFI/WASM envelope shape
/// (`DiagnosticJson` — all 9 fields always present, lowercase severity).
fn diagnostic_to_json_value(d: &crate::error::Diagnostic) -> serde_json::Value {
    serde_json::to_value(crate::error::DiagnosticJson::from(d)).unwrap_or_else(|_| {
        serde_json::json!({
            "severity": "error",
            "code": "E_SERIALIZE",
            "message": "failed to serialize diagnostic",
            "file": null,
            "line": 0,
            "col": 0,
            "end_line": null,
            "end_col": null,
            "suggestion": null,
        })
    })
}

fn rust_to_c_string(s: String) -> *mut c_char {
    // If the string contains an interior NUL byte, replace it
    // with a placeholder rather than silently returning "{}"
    let sanitized = s.replace('\0', "\\0");
    CString::new(sanitized)
        .unwrap_or_else(|_| {
            CString::new("{\"error\":\"serialization failed\"}")
                .expect("hardcoded CString has no interior NUL bytes")
        })
        .into_raw()
}

/// Free a string previously returned by an omni_* function.
/// # Safety
/// `ptr` must be a valid pointer previously returned by an omni_* function, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: `ptr` is non-null (checked above) and was produced by
    // CString::into_raw() from a prior omni_* call. We take ownership
    // back via from_raw() so the CString is dropped and its memory freed.
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Return compiler version as C string (caller must free).
#[no_mangle]
pub extern "C" fn omni_version() -> *mut c_char {
    rust_to_c_string(env!("CARGO_PKG_VERSION").to_string())
}

/// Analyze multiple .omni sources and return diagnostics (errors, warnings, lint results).
///
/// Takes JSON: `{ "sources": { "path": "source", ... }, "path_mapping": { "orig": "resolved", ... } }`
/// Returns JSON: `{ "success": bool, "diagnostics": [...] }`
///
/// # Safety
/// `request_json_ptr` must be valid for `request_json_len` bytes, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_analyse(
    request_json_ptr: *const u8,
    request_json_len: u32,
) -> *mut c_char {
    let request_str = if request_json_ptr.is_null() {
        let err_json = serde_json::json!({
            "success": false,
            "diagnostics": [crate::error::DiagnosticJson::synthetic("E0000", "null pointer")],
        });
        return rust_to_c_string(err_json.to_string());
    } else {
        // SAFETY: `request_json_ptr` is non-null (checked above) and the
        // caller guarantees it points to `request_json_len` bytes of valid
        // UTF-8 data for the duration of this call.
        let slice =
            unsafe { std::slice::from_raw_parts(request_json_ptr, request_json_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                let err_json = serde_json::json!({
                    "success": false,
                    "diagnostics": [crate::error::DiagnosticJson::synthetic(
                        "E0000",
                        format!("Invalid UTF-8 input: {}", e),
                    )],
                });
                return rust_to_c_string(err_json.to_string());
            }
        }
    };

    #[derive(Deserialize)]
    struct AnalyseRequest {
        sources: FxHashMap<String, String>,
        #[serde(default)]
        path_mapping: FxHashMap<String, String>,
    }

    let input: AnalyseRequest = match serde_json::from_str(request_str) {
        Ok(req) => req,
        Err(e) => {
            let err_json = serde_json::json!({
                "success": false,
                "diagnostics": [crate::error::DiagnosticJson::synthetic(
                    "E0000",
                    format!("Failed to parse request JSON: {}", e),
                )],
            });
            return rust_to_c_string(err_json.to_string());
        }
    };

    // Run the unified analysis pipeline
    let analysis = crate::analyse::analyse_multi(&input.sources, &input.path_mapping);

    // Convert Diagnostics to JSON Value objects matching FFI output format
    let mut diagnostics: Vec<serde_json::Value> = Vec::new();

    for d in &analysis.diagnostics {
        diagnostics.push(diagnostic_to_json_value(d));
    }

    for w in &analysis.warnings {
        diagnostics.push(diagnostic_to_json_value(w));
    }

    let response = serde_json::json!({
        "success": diagnostics.iter().all(|d| d["severity"] != "error"),
        "diagnostics": diagnostics,
    });

    rust_to_c_string(response.to_string())
}

// omni_compile_multi_to_target moved to the omnilang facade crate.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Diagnostic, DiagnosticJson, DiagnosticKind, Severity};
    use crate::lexer::Span;

    /// Invoke `omni_analyse` with a JSON request and return the parsed response.
    fn call_omni_analyse(request: &str) -> serde_json::Value {
        let bytes = request.as_bytes();
        // SAFETY: `bytes` outlives the call; pointer+len are valid for the
        // whole call (test-only).
        let ptr = unsafe { omni_analyse(bytes.as_ptr(), bytes.len() as u32) };
        assert!(!ptr.is_null(), "omni_analyse returned null");
        // SAFETY: `ptr` came from omni_analyse and is freed exactly once.
        let s = unsafe { CString::from_raw(ptr) };
        let s = s.to_string_lossy().into_owned();
        serde_json::from_str(&s).expect("response is valid JSON")
    }

    #[test]
    fn diagnostic_to_json_value_matches_unified_shape() {
        let d = Diagnostic::error(
            DiagnosticKind::E0001,
            "bad token",
            &Span {
                start: 0,
                end: 4,
                line: 3,
                col: 5,
            },
        )
        .with_file("main.omni")
        .with_suggestion("try again");
        let json = diagnostic_to_json_value(&d);
        assert_eq!(
            json,
            serde_json::json!({
                "severity": "error",
                "code": "E0001",
                "message": "bad token",
                "file": "main.omni",
                "line": 3,
                "col": 5,
                "end_line": 3,
                "end_col": 9,
                "suggestion": "try again",
            })
        );
    }

    #[test]
    fn omni_analyse_diagnostics_have_unified_shape() {
        let request = serde_json::json!({
            "sources": {
                "main.omni": "this is not valid omni @@@"
            },
            "path_mapping": {},
        });
        let response = call_omni_analyse(&request.to_string());
        assert!(!response["success"].as_bool().unwrap());
        let diagnostics = response["diagnostics"].as_array().unwrap();
        assert!(!diagnostics.is_empty());
        let mut keys: Vec<&str> = diagnostics[0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        let expected = [
            "code",
            "col",
            "end_col",
            "end_line",
            "file",
            "line",
            "message",
            "severity",
            "suggestion",
        ];
        let mut expected = expected.to_vec();
        expected.sort_unstable();
        assert_eq!(
            keys, expected,
            "diagnostic envelope must have the unified 9-field shape"
        );
        // Severity is lowercase — the old "Error" capitalization is gone.
        assert_eq!(diagnostics[0]["severity"], "error");
        // Explicit nulls, not omitted keys, for missing optional fields.
        assert!(diagnostics[0]["file"].is_string());
        assert!(diagnostics[0]["suggestion"].is_null());
    }

    #[test]
    fn omni_analyse_synthetic_errors_use_unified_shape() {
        // Empty/blank request → boundary error envelope (unified shape).
        let response = call_omni_analyse("");
        let d = &response["diagnostics"][0];
        assert_eq!(d["severity"], "error");
        assert_eq!(d["code"], "E0000");
        assert!(d["end_line"].is_null());
        assert!(d["end_col"].is_null());
        assert!(d["suggestion"].is_null());

        // Malformed JSON request → unified envelope, lowercase severity
        // (the old capitalized "Error" is a contract violation).
        let response = call_omni_analyse("{ not json");
        let d = &response["diagnostics"][0];
        assert_eq!(d["severity"], "error");
        assert_eq!(d["code"], "E0000");
        assert!(d["file"].is_null());
    }

    #[test]
    fn omni_analyse_warnings_use_unified_shape() {
        // An unused import triggers a lint-level warning — it must surface
        // in the diagnostics array with the unified shape (lowercase
        // severity, all 9 fields present with explicit nulls).
        let request = serde_json::json!({
            "sources": {
                "main.omni": "import { array_length } from \"std/array\";\ncomponent App { view { <div>hi</div> } }"
            },
            "path_mapping": {},
        });
        let response = call_omni_analyse(&request.to_string());
        let diagnostics = response["diagnostics"].as_array().unwrap();
        assert!(
            diagnostics.iter().any(|d| d["severity"] == "warning"),
            "expected at least one warning diagnostic; got: {diagnostics:?}"
        );
        for d in diagnostics {
            assert_eq!(
                d["severity"].as_str().unwrap().to_lowercase(),
                d["severity"]
            );
            for field in [
                "code",
                "message",
                "file",
                "line",
                "col",
                "end_line",
                "end_col",
                "suggestion",
            ] {
                assert!(d.get(field).is_some(), "missing field {field}: {d}");
            }
        }
    }

    #[test]
    fn diagnostic_json_from_warning_keeps_severity() {
        let d = Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::WMotion,
            message: "motion warning".into(),
            file: Some("a.omni".into()),
            line: 2,
            col: 3,
            end_line: None,
            end_col: None,
            suggestion: None,
        };
        let json = serde_json::to_value(DiagnosticJson::from(&d)).unwrap();
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["code"], "W0001");
        assert_eq!(json["file"], "a.omni");
        assert_eq!(json["line"], 2);
    }
}
