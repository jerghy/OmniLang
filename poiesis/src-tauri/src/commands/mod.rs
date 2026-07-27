//! Re-export all Tauri command functions for registration in lib.rs.

pub mod analyse;
pub mod compile;
pub mod debug;
pub mod project;
pub mod state_sync;

pub(crate) use analyse::*;
pub(crate) use compile::*;
pub(crate) use debug::*;
pub(crate) use project::*;
pub(crate) use state_sync::*;
