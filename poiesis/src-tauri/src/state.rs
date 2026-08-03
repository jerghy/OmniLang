//! Application state managed by Tauri.
//!
//! P18(Sprint 5): extracted from lib.rs.

#[cfg(feature = "server-compile")]
use rustc_hash::FxHashMap;
use std::sync::Mutex;
#[cfg(feature = "server-compile")]
use std::time::SystemTime;

pub(crate) struct CachedFileList {
    #[cfg(feature = "server-compile")]
    pub sources: FxHashMap<String, String>,
    #[cfg(feature = "server-compile")]
    pub root_mtime: Option<SystemTime>,
}

pub(crate) struct ProjectState {
    pub project_dir: Mutex<Option<String>>,
    pub file_list_cache: Mutex<Option<CachedFileList>>,
    #[cfg(feature = "server-compile")]
    pub compile_cache: Mutex<omnilang::analyse::IncrementalCache>,
    pub store_snapshot: Mutex<String>,
    pub pid_index: Mutex<String>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            project_dir: Mutex::new(None),
            file_list_cache: Mutex::new(None),
            #[cfg(feature = "server-compile")]
            compile_cache: Mutex::new(omnilang::analyse::IncrementalCache::new()),
            store_snapshot: Mutex::new(String::new()),
            pid_index: Mutex::new("{}".to_string()),
        }
    }
}
