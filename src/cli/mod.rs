//! CLI entry: argument parsing, single-file and batch processing (native only).

#![cfg(not(target_arch = "wasm32"))]

mod args;
mod batch;

pub use args::{parse_cli_args, run_cli, CliCommand};
pub use batch::{
    collect_batch_inputs, get_output_path, is_supported_image_path, print_processed_image,
    process, process_batch, process_batch_with_reporter, process_file, process_single,
    BatchConfig, BatchEvent,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod cli_tests;
