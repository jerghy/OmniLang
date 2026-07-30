//! Debug bridge emission for standalone debug window.
//! When debug_mode is enabled, emits JS that communicates with
//! the main Poiesis window via Tauri events (window.__TAURI_INTERNALS__).

use super::OmniViewGenerator;

impl OmniViewGenerator {
    /// Emit debug bridge code for the standalone debug window.
    /// Uses Tauri IPC (window.__TAURI_INTERNALS__) instead of parent.postMessage.
    /// Called after emit_editor_bridge(), before finalize_js().
    pub(super) fn emit_debug_bridge(&mut self) {
        if !self.debug_mode {
            return;
        }
        self.js.push_str(include_str!("bridges/debug_bridge.js"));
    }
}
