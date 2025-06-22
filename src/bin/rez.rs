//! Rez Core CLI Tool
//!
//! A high-performance command-line interface for the Rez package manager,
//! built with Rust for optimal performance.

use clap::Parser;
use std::process;
use std::env;

// Import CLI from the library
use rez_core::cli::{RezCli, RezCommand};

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if this is a command that supports grouped arguments (with '--')
    if args.len() > 1 && (args[1] == "env" || args[1] == "build") {
        handle_grouped_command(args);
    } else {
        // Standard argument parsing
        let cli = RezCli::parse();
        if let Err(e) = cli.run() {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn handle_grouped_command(args: Vec<String>) {
    // Split arguments by '--'
    let mut arg_groups = vec![vec![]];
    let mut current_group = 0;

    for arg in args.iter().skip(1) { // Skip program name
        if arg == "--" {
            arg_groups.push(vec![]);
            current_group += 1;
        } else {
            arg_groups[current_group].push(arg.clone());
        }
    }

    // Parse the first group normally
    let mut cli_args = vec![args[0].clone()]; // Add program name back
    cli_args.extend(arg_groups[0].clone());

    match RezCli::try_parse_from(cli_args) {
        Ok(mut cli) => {
            // Handle extra arguments for specific commands
            if let Some(ref mut command) = cli.command {
                match command {
                    RezCommand::Env(ref mut env_args) => {
                        if arg_groups.len() > 1 && !arg_groups[1].is_empty() {
                            if let Err(e) = rez_core::cli::commands::env::execute_with_extra_args(
                                env_args.clone(),
                                arg_groups[1].clone()
                            ) {
                                eprintln!("Error: {}", e);
                                process::exit(1);
                            }
                            return;
                        }
                    }
                    RezCommand::Build(_) => {
                        // TODO: Handle build command extra args
                    }
                    _ => {}
                }
            }

            // Execute normally if no extra args
            if let Err(e) = cli.run() {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            process::exit(1);
        }
    }
}


