//! Codegen trait -- implement for each target.

use rustc_hash::FxHashMap;
use crate::ir::IRProgram;

use super::CompileOutput;

/// Codegen trait -- implement for each target.
pub trait Codegen: Send + Sync {
    /// Generate output files for the given IR program and source files.
    #[must_use]
    fn generate(&self, ir: &IRProgram, source_files: &FxHashMap<String, String>) -> CompileOutput;
}
