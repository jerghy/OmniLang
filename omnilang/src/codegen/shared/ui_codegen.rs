//! Shared trait for UI-target code generators.
//!
//! `UiCodegen` captures the common interface of HTML/CSS/JS framework
//! targets (React TSX, Vue SFC, OmniView JS/HTML/CSS) that is outside
//! the scope of the universal [`Codegen`](super::super::traits::Codegen) trait.
//!
//! # Backends
//!
//! | Backend | Struct | Feature flag |
//! |---------|--------|-------------|
//! | React   | `ReactCodegen` | `codegen-react` |
//! | Vue     | `VueCodegen` | `codegen-vue` |
//! | OmniView | `OmniViewGenerator` | `codegen-omniview` |
//!
//! # Thin Delegation
//!
//! Implementations delegate to existing methods on each backend —
//! code is NOT moved into the trait methods (that is a follow-up refactor).

use crate::ir::{ComponentIR, IRProgram, StoreIR};

/// Trait for UI-target code generators (React, OmniView, Vue, etc.).
///
/// Provides a common interface for generating component UI source code,
/// component CSS, and store code across HTML/CSS/JS framework targets.
///
/// This trait is **separate** from [`Codegen`](super::super::traits::Codegen):
/// `Codegen` is the universal interface for all 10+ codegen targets, while
/// `UiCodegen` captures UI-specific concerns (components with views, stores
/// with signals/getters/actions, CSS output) shared only by UI frameworks.
pub trait UiCodegen {
    /// Generate the UI component source code as a string.
    ///
    /// Returns the framework-specific component code (e.g., TSX for React,
    /// `.vue` SFC for Vue, JS factory for OmniView).
    fn generate_component_code(&self, name: &str, comp: &ComponentIR, ir: &IRProgram) -> String;

    /// Generate CSS for a component.
    ///
    /// Returns an empty string by default (when CSS is embedded inline,
    /// e.g., Vue SFC `<style scoped>` block).
    fn generate_component_css(&self, name: &str, comp: &ComponentIR) -> String {
        let _ = (name, comp);
        String::new()
    }

    /// Generate store code as a string.
    ///
    /// Returns the framework-specific store code (e.g., React custom hook,
    /// Vue composable, or OmniView global store JS).
    fn generate_store_code(&self, name: &str, store: &StoreIR) -> String;

    /// Primary file extension for generated component files (e.g., `".tsx"`,
    /// `".vue"`, `".js"`).
    fn component_file_extension(&self) -> &str;

    /// File extension for generated store files.
    ///
    /// Defaults to [`component_file_extension`](UiCodegen::component_file_extension).
    fn store_file_extension(&self) -> &str {
        self.component_file_extension()
    }
}

