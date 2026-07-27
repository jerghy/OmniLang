//! Native C FFI (for Python ctypes/cffi).
//!
//! Functions: omni_free_string, omni_version, omni_analyse.
//!
//! Note: omni_compile_multi_to_target moved to the omnilang facade crate.

use rustc_hash::FxHashMap;
use std::ffi::CString;
use std::os::raw::c_char;

use serde::Deserialize;

fn diagnostic_to_json_value(d: &crate::error::Diagnostic) -> serde_json::Value {
    serde_json::json!({
        "severity": match d.severity {
            crate::error::Severity::Error => "error",
            crate::error::Severity::Warning => "warning",
            crate::error::Severity::Info => "info",
        },
        "code": d.kind.code_str(),
        "message": d.message,
        "file": d.file,
        "line": d.line,
        "col": d.col,
        "suggestion": d.suggestion,
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
        return rust_to_c_string(
            r#"{"success":false,"diagnostics":[{"message":"null pointer","severity":"error"}]}"#
                .into(),
        );
    } else {
        // SAFETY: `request_json_ptr` is non-null (checked above) and the
        // caller guarantees it points to `request_json_len` bytes of valid
        // UTF-8 data for the duration of this call.
        let slice =
            unsafe { std::slice::from_raw_parts(request_json_ptr, request_json_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                return rust_to_c_string(
                    serde_json::json!({
                        "success": false,
                        "diagnostics": [{
                            "severity": "error",
                            "code": "E0000",
                            "message": format!("Invalid UTF-8 input: {}", e),
                            "line": 0,
                            "col": 0
                        }]
                    })
                    .to_string(),
                );
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
                "diagnostics": [{
                    "severity": "Error",
                    "code": "E0000",
                    "message": format!("Failed to parse request JSON: {}", e),
                    "line": 0,
                    "col": 0
                }],
                "success": false
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
        diagnostics.push(serde_json::json!({
            "severity": "warning",
            "code": w.kind.code_str(),
            "message": w.message,
            "file": w.file,
            "line": w.line,
            "col": w.col,
            "suggestion": null,
        }));
    }

    let response = serde_json::json!({
        "success": diagnostics.iter().all(|d| d["severity"] != "error"),
        "diagnostics": diagnostics,
    });

    rust_to_c_string(response.to_string())
}

// omni_compile_multi_to_target moved to the omnilang facade crate.
