//! Placeholder for the flutter_rust_bridge output.
//!
//! `flutter_rust_bridge_codegen generate` overwrites this file. Until it has
//! run, the `frb` feature cannot build, so say why rather than failing with a
//! missing-module error.

compile_error!(
    "src/frb_generated.rs has not been generated yet; run `flutter_rust_bridge_codegen generate` \
     from patto-flutter/ before building with the `frb` feature"
);
