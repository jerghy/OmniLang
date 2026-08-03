//! Re-export all Tauri command functions for registration in lib.rs.

#[cfg(feature = "server-compile")]
pub mod analyse;
#[cfg(feature = "server-compile")]
pub mod compile;
pub mod debug;
pub mod project;
pub mod state_sync;

#[cfg(feature = "server-compile")]
pub(crate) use analyse::*;
#[cfg(feature = "server-compile")]
pub(crate) use compile::*;
pub(crate) use debug::*;
pub(crate) use project::*;
pub(crate) use state_sync::*;
