//! Patto Flutter Rust bindings
//!
//! This crate provides Rust functionality to the Patto Flutter app via flutter_rust_bridge.
//! It includes:
//! - Parser bindings (wrapping the main patto crate)
//! - Git operations (using git2-rs)

pub mod api;

pub use api::*;
