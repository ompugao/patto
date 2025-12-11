//! Patto Flutter Rust bindings
//!
//! This crate provides Rust functionality to the Patto Flutter app via flutter_rust_bridge.
//! It includes:
//! - Parser bindings (wrapping the main patto crate)
//! - Git operations (using git2-rs)

mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */

pub mod api;

pub use api::*;
