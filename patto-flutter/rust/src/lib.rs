//! Rust core of the Patto notes mobile app.
//!
//! `api` holds the logic and is plain Rust: it builds and unit-tests on the host
//! with no Flutter toolchain. `frb_api` is the thin surface flutter_rust_bridge
//! generates Dart bindings for; it needs the generated `frb_generated` module,
//! so it lives behind the `frb` feature that the app build turns on.

pub mod api;

#[cfg(feature = "frb")]
pub mod frb_api;
#[cfg(feature = "frb")]
mod frb_generated;
