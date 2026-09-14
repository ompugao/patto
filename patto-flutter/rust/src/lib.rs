//! Rust core of the Patto notes mobile app.
//!
//! `api` holds the logic and is plain Rust, so it builds and unit-tests on the
//! host with no Flutter toolchain. `frb_api` is the thin surface
//! flutter_rust_bridge generates Dart bindings for.

pub mod api;
pub mod frb_api;

mod frb_generated;
