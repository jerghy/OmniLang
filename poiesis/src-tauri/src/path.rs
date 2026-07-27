//! Path validation utilities — extracted from lib.rs (P18).

/// Validate and canonicalize a user-supplied relative path against a base directory.
/// Returns the canonicalized path if it is within the base directory.
/// Resolves symlinks, `..`, and other path traversal attempts.
///
/// # Safety note (TOCTOU)
/// There is a time-of-check-time-of-use window between path validation and
/// the subsequent file operation. This is acceptable because the project
/// directory is assumed to be under the user's control and not shared with
/// untrusted writers. On production deployments, consider using handle-based
/// operations (e.g. O_NOFOLLOW on Unix) for stronger guarantees.
#[must_use = "sanitized path must be used or path traversal checks are bypassed"]
pub(crate) fn sanitize_path(
    base: &std::path::Path,
    user_path: &str,
) -> Result<std::path::PathBuf, String> {
    // Reject empty paths
    if user_path.is_empty() {
        return Err("Empty path is not allowed".to_string());
    }

    // Reject paths that are already absolute
    let p = std::path::Path::new(user_path);
    if p.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }

    // Join with base and canonicalize (resolves symlinks, .., etc.)
    let full = base.join(p);

    // Canonicalize the base for comparison (must exist)
    let base_canonical = base
        .canonicalize()
        .map_err(|e| format!("Base directory error: {}", e))?;

    // Try canonicalizing the full path first (works for existing files)
    // If the file doesn't exist yet, canonicalize the parent directory
    // and append the filename (handles write_file creating new files)
    let resolved = full.canonicalize().or_else(|_| {
        full.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .map(|parent_canon| {
                parent_canon.join(full.file_name().unwrap_or(std::ffi::OsStr::new("unknown")))
            })
            .ok_or_else(|| format!("Invalid path (cannot resolve): {}", full.display()))
    })?;

    // Verify the resolved path is within the base directory
    if !resolved.starts_with(&base_canonical) {
        return Err("Path traversal detected".to_string());
    }

    Ok(resolved)
}
