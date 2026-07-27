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
        return rust_to_c_string(
            r#"{"success":false,"files":{},"errors":["null sources pointer"]}"#.into(),
        );
    } else {
        // SAFETY: `sources_json_ptr` is non-null (checked above) and the
        // caller guarantees it points to `sources_json_len` bytes of valid
        // UTF-8 data for the duration of this call.
        let slice =
            unsafe { std::slice::from_raw_parts(sources_json_ptr, sources_json_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                return rust_to_c_string(
                    serde_json::json!({
                        "success": false,
                        "files": {},
                        "errors": [format!("Invalid UTF-8 input for sources: {}", e)]
                    })
                    .to_string(),
                );
            }
        }
    };

    let target_str = if target_ptr.is_null() {
        return rust_to_c_string(
            r#"{"success":false,"files":{},"errors":["null target pointer"]}"#.into(),
        );
    } else {
        // SAFETY: `target_ptr` is non-null (checked above) and the caller
        // guarantees it points to `target_len` bytes of valid UTF-8 data.
        let slice = unsafe { std::slice::from_raw_parts(target_ptr, target_len as usize) };
        match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                return rust_to_c_string(
                    serde_json::json!({
                        "success": false,
                        "files": {},
                        "errors": [format!("Invalid UTF-8 input for target: {}", e)]
                    })
                    .to_string(),
                );
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
                return rust_to_c_string(
                    serde_json::json!({
                        "success": false,
                        "files": {},
                        "errors": [format!("Invalid UTF-8 input for path_mapping: {}", e)]
                    })
                    .to_string(),
                );
            }
        }
    };

    // Parse sources
    let sources: FxHashMap<String, String> = match serde_json::from_str(sources_str) {
        Ok(s) => s,
        Err(e) => {
            return rust_to_c_string(
                serde_json::json!({
                    "success": false,
                    "files": {},
                    "errors": [format!("Failed to parse sources JSON: {}", e)]
                })
                .to_string(),
            );
        }
    };

    // Parse target
    let target: crate::codegen::Target = match target_str.parse() {
        Ok(t) => t,
        Err(e) => {
            return rust_to_c_string(
                serde_json::to_string(&serde_json::json!({
                    "success": false,
                    "files": {},
                    "errors": [e]
                }))
                .unwrap_or_else(|_| {
                    r#"{"success":false,"files":{},"errors":["serialization error"]}"#.into()
                }),
            );
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
                return rust_to_c_string(
                    serde_json::json!({
                        "success": false,
                        "files": {},
                        "errors": [format!("Invalid UTF-8 input for editor_mode: {}", e)]
                    })
                    .to_string(),
                );
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
                "errors": [{"severity":"error","code":"E_SERIALIZE","message":format!("Failed to deserialize compile result: {}", e)}]
            }),
        };
        response["warnings"] = serde_json::json!(warnings);
        rust_to_c_string(response.to_string())
    }
}
