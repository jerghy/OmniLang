//! Native C FFI (for Python ctypes/cffi).
//!
//! Functions: omni_compile_multi_to_target.
//!
//! Note: omni_free_string, omni_version, omni_analyse are provided by omnilang-core.

use rustc_hash::FxHashMap;
use std::ffi::CString;
use std::os::raw::c_char;

use crate::compile_multi_to_target;

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

/// Build a C-FFI boundary-error response envelope.
///
/// Errors use the unified 9-field `DiagnosticJson` shape so the C-FFI
/// boundary matches the WASM boundary (`omni_analyse`).
fn boundary_error_envelope(message: impl Into<String>) -> String {
    serde_json::json!({
        "success": false,
        "files": {},
        "errors": [crate::error::DiagnosticJson::synthetic("E0000", message)],
    })
    .to_string()
}

/// Compile multiple .omni sources to a target language via C FFI.
///
/// # Parameters (ptr + len convention for all string inputs)
/// - `sources_json_ptr/len`: JSON object mapping file paths to source text
/// - `target_ptr/len`: target language string (e.g. "python", "js", "rust", "dart")
/// - `path_mapping_ptr/len`: JSON object mapping original import paths to resolved paths
///
/// Returns a JSON string (caller must free with omni_free_string from omnilang-core).
/// # Safety
/// All pointers must be valid for the given length, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_compile_multi_to_target(
    sources_json_ptr: *const u8,
    sources_json_len: u32,
    target_ptr: *const u8,
    target_len: u32,
    path_mapping_ptr: *const u8,
    path_mapping_len: u32,
    editor_mode_ptr: *const u8,
    editor_mode_len: u32,
) -> *mut c_char {
    // Convert ptr+len to string slices
    let sources_str = if sources_json_ptr.is_null() {
        return rust_to_c_string(boundary_error_envelope("null sources pointer"));
    } else {
        // SAFETY: `sources_json_ptr` is non-null (checked above) and the
        // caller guarantees it points to `sources_json_len` bytes of valid
        // UTF-8 data for the duration of this call.
        let slice =
            unsafe { std::slice::from_raw_parts(sources_json_ptr, sources_json_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                return rust_to_c_string(boundary_error_envelope(format!(
                    "Invalid UTF-8 input for sources: {}",
                    e
                )));
            }
        }
    };

    let target_str = if target_ptr.is_null() {
        return rust_to_c_string(boundary_error_envelope("null target pointer"));
    } else {
        // SAFETY: `target_ptr` is non-null (checked above) and the caller
        // guarantees it points to `target_len` bytes of valid UTF-8 data.
        let slice = unsafe { std::slice::from_raw_parts(target_ptr, target_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                return rust_to_c_string(boundary_error_envelope(format!(
                    "Invalid UTF-8 input for target: {}",
                    e
                )));
            }
        }
    };

    let path_mapping_str = if path_mapping_ptr.is_null() {
        "{}"
    } else {
        // SAFETY: `path_mapping_ptr` is non-null (implicit from else branch)
        // and the caller guarantees it points to `path_mapping_len` bytes.
        let slice =
            unsafe { std::slice::from_raw_parts(path_mapping_ptr, path_mapping_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                return rust_to_c_string(boundary_error_envelope(format!(
                    "Invalid UTF-8 input for path_mapping: {}",
                    e
                )));
            }
        }
    };

    // Parse sources
    let sources: FxHashMap<String, String> = match serde_json::from_str(sources_str) {
        Ok(s) => s,
        Err(e) => {
            return rust_to_c_string(boundary_error_envelope(format!(
                "Failed to parse sources JSON: {}",
                e
            )));
        }
    };

    // Parse target
    let target: crate::codegen::Target = match target_str.parse() {
        Ok(t) => t,
        Err(e) => {
            return rust_to_c_string(boundary_error_envelope(format!(
                "Failed to parse target: {}",
                e
            )));
        }
    };

    let mut warnings: Vec<String> = Vec::new();

    // Parse path_mapping (optional — defaults to empty on parse failure)
    let path_mapping: FxHashMap<String, String> = match serde_json::from_str(path_mapping_str) {
        Ok(m) => m,
        Err(e) => {
            warnings.push(format!("Warning: failed to parse path_mapping JSON: {}", e));
            FxHashMap::default()
        }
    };

    // Parse editor_mode
    let editor_mode = if editor_mode_ptr.is_null() {
        false
    } else {
        // SAFETY: editor_mode_ptr is non-null and editor_mode_len bytes long, guaranteed by FFI caller contract.
        let slice =
            unsafe { std::slice::from_raw_parts(editor_mode_ptr, editor_mode_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s.trim() == "true",
            Err(e) => {
                return rust_to_c_string(boundary_error_envelope(format!(
                    "Invalid UTF-8 input for editor_mode: {}",
                    e
                )));
            }
        }
    };

    let result = compile_multi_to_target(sources, target, path_mapping, editor_mode, false, None);
    if warnings.is_empty() {
        rust_to_c_string(result)
    } else {
        let mut response: serde_json::Value = match serde_json::from_str(&result) {
            Ok(v) => v,
            Err(e) => serde_json::json!({
                "success": false,
                "files": {},
                "errors": [crate::error::DiagnosticJson::synthetic(
                    "E_SERIALIZE",
                    format!("Failed to deserialize compile result: {}", e),
                )]
            }),
        };
        response["warnings"] = serde_json::json!(warnings);
        rust_to_c_string(response.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invoke `omni_compile_multi_to_target` with byte slices and return the parsed response.
    fn call_compile(sources: &[u8], target: &[u8]) -> serde_json::Value {
        // SAFETY: `sources`/`target` outlive the call and their pointer+len
        // are valid for the whole call (test-only).
        let ptr = unsafe {
            omni_compile_multi_to_target(
                sources.as_ptr(),
                sources.len() as u32,
                target.as_ptr(),
                target.len() as u32,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
            )
        };
        assert!(!ptr.is_null(), "omni_compile_multi_to_target returned null");
        // SAFETY: `ptr` came from omni_compile_multi_to_target and is freed exactly once.
        let s = unsafe { CString::from_raw(ptr) };
        serde_json::from_str(&s.to_string_lossy()).expect("response is valid JSON")
    }

    #[test]
    fn c_ffi_boundary_errors_are_structured() {
        // Empty sources slice → early-return boundary error envelope.
        let out = call_compile(&[], b"react");
        assert!(!out["success"].as_bool().unwrap());
        let errors = out["errors"].as_array().expect("errors must be an array");
        assert!(!errors.is_empty());
        for e in errors {
            assert!(
                e.get("code").is_some() && e.get("line").is_some(),
                "raw string envelope: {e}"
            );
        }
        // Unified 9-field shape: lowercase severity, explicit nulls.
        let d = &errors[0];
        assert_eq!(d["severity"], "error");
        assert_eq!(d["code"], "E0000");
        assert!(d["file"].is_null());
        assert!(d["end_line"].is_null());
        assert!(d["end_col"].is_null());
        assert!(d["suggestion"].is_null());
        let mut keys: Vec<&str> = d.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        let mut expected = [
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
        expected.sort_unstable();
        assert_eq!(
            keys, expected,
            "diagnostic envelope must have the unified 9-field shape"
        );
    }

    #[test]
    fn c_ffi_null_pointer_error_is_structured() {
        // Null sources pointer → null-pointer boundary envelope.
        // SAFETY: all pointers null with len 0; the function handles null internally.
        let ptr = unsafe {
            omni_compile_multi_to_target(
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
            )
        };
        assert!(!ptr.is_null(), "omni_compile_multi_to_target returned null");
        // SAFETY: `ptr` came from the function and is freed exactly once.
        let s = unsafe { CString::from_raw(ptr) };
        let out: serde_json::Value = serde_json::from_str(&s.to_string_lossy()).unwrap();
        let d = &out["errors"][0];
        assert_eq!(d["severity"], "error");
        assert_eq!(d["code"], "E0000");
        assert!(d["message"]
            .as_str()
            .unwrap()
            .contains("null sources pointer"));
    }
}
