//! Rez Core CLI Tool
//!
//! A high-performance command-line interface for the Rez package manager,
//! built with Rust for optimal performance.

use clap::Parser;
use std::process;

// Import CLI from the library
use rez_core::cli::RezCli;

fn main() {
    // Parse command line arguments
    let cli = RezCli::parse();

    // Execute the CLI application
    if let Err(e) = cli.run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}


