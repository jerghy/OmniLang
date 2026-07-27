//! Application state managed by Tauri.
//!
//! P18(Sprint 5): extracted from lib.rs.

use rustc_hash::FxHashMap;
use std::sync::Mutex;
use std::time::SystemTime;

pub(crate) struct CachedFileList {
    pub sources: FxHashMap<String, String>,
    pub root_mtime: Option<SystemTime>,
}

pub(crate) struct ProjectState {
    pub project_dir: Mutex<Option<String>>,
    pub file_list_cache: Mutex<Option<CachedFileList>>,
    pub compile_cache: Mutex<omnilang::analyse::IncrementalCache>,
    pub store_snapshot: Mutex<String>,
    pub pid_index: Mutex<String>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            project_dir: Mutex::new(None),
            file_list_cache: Mutex::new(None),
            compile_cache: Mutex::new(omnilang::analyse::IncrementalCache::new()),
            store_snapshot: Mutex::new(String::new()),
            pid_index: Mutex::new("{}".to_string()),
        }
    }
}
