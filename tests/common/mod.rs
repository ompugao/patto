//! Helpers shared by the LSP integration tests.
//!
//! Every test binary compiles this module separately and uses only the part it
//! needs, so unused items here are expected.
#![allow(dead_code, unused_imports)]

mod assertions;
mod in_process_client;
mod workspace;

pub use assertions::*;
pub use in_process_client::InProcessLspClient;
pub use workspace::TestWorkspace;
