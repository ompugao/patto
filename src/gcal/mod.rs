//! Google Calendar synchronization module for Patto tasks.
//!
//! This module provides functionality to sync Patto task deadlines
//! to Google Calendar events.

#[cfg(feature = "gcal")]
pub mod auth;
#[cfg(feature = "gcal")]
pub mod config;
#[cfg(feature = "gcal")]
pub mod event_mapper;
#[cfg(feature = "gcal")]
pub mod state;
#[cfg(feature = "gcal")]
pub mod sync;

#[cfg(feature = "gcal")]
pub use auth::*;
#[cfg(feature = "gcal")]
pub use config::*;
#[cfg(feature = "gcal")]
pub use event_mapper::*;
#[cfg(feature = "gcal")]
pub use state::*;
#[cfg(feature = "gcal")]
pub use sync::*;
