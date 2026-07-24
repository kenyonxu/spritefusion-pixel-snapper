//! CLI entry: argument parsing, single-file and batch processing (native only).

#![cfg(not(target_arch = "wasm32"))]

mod args;
mod batch;

// Only these two are re-exported: `run_cli` is re-exported by lib.rs (the
// src/main.rs shim), and args.rs calls `crate::cli::process` internally.
// cli_tests imports from `super::args` directly, so the other `pub use`s
// were dead weight.
pub use args::run_cli;
pub use batch::process;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod cli_tests;
