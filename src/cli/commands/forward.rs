//! # Forward Command
//!
//! The `rez forward` command provides a compatibility shim that forwards
//! rez commands to rez_next. This allows existing scripts using `rez` to
//! seamlessly use rez_next under the hood.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use std::process::Command;

/// Arguments for the forward command
#[derive(Args, Clone)]
pub struct ForwardArgs {
    /// Print the resolved rez_next binary path and exit
    #[arg(long)]
    pub print_path: bool,

    /// Dry run - print what would be executed without running
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Arguments to forward to rez_next
    #[arg(value_name = "ARGS", allow_hyphen_values = true, trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// Execute the forward command
pub fn execute(args: ForwardArgs) -> RezCoreResult<()> {
    // Find the rez_next binary
    let rez_next_bin = find_rez_next_binary();

    if args.print_path {
        println!("{}", rez_next_bin);
        return Ok(());
    }

    if args.dry_run {
        println!("Would execute: {} {}", rez_next_bin, args.args.join(" "));
        return Ok(());
    }

    if args.args.is_empty() {
        // No args: just show help
        println!("Usage: rez forward [OPTIONS] [ARGS]...");
        println!("Forward rez commands to rez_next. Append any rez command and its arguments.");
        println!();
        println!("Examples:");
        println!("  rez forward env python-3.9");
        println!("  rez forward search maya");
        println!("  rez forward --dry-run env python-3.9 maya-2023");
        return Ok(());
    }

    // Execute rez_next with the forwarded arguments
    let status = Command::new(&rez_next_bin)
        .args(&args.args)
        .status()
        .map_err(|e| {
            RezCoreError::ExecutionError(format!(
                "Failed to execute rez_next ({}): {}",
                rez_next_bin, e
            ))
        })?;

    let code = status.code().unwrap_or(1);
    if code != 0 {
        std::process::exit(code);
    }

    Ok(())
}

/// Find the rez_next binary path
fn find_rez_next_binary() -> String {
    // 1. REZ_NEXT_BIN environment variable
    if let Ok(bin) = std::env::var("REZ_NEXT_BIN") {
        if !bin.is_empty() {
            return bin;
        }
    }

    // 2. Same directory as current executable
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().unwrap_or(std::path::Path::new("."));
        let rez_next = if cfg!(windows) {
            dir.join("rez-next.exe")
        } else {
            dir.join("rez-next")
        };
        if rez_next.exists() {
            return rez_next.to_string_lossy().to_string();
        }
    }

    // 3. Fallback: use PATH
    if cfg!(windows) {
        "rez-next.exe".to_string()
    } else {
        "rez-next".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_find_binary() {
        let bin = find_rez_next_binary();
        assert!(!bin.is_empty());
    }

    #[test]
    fn test_forward_print_path() {
        let args = ForwardArgs {
            print_path: true,
            dry_run: false,
            args: vec![],
        };
        assert!(execute(args).is_ok());
    }

    #[test]
    fn test_forward_dry_run() {
        let args = ForwardArgs {
            print_path: false,
            dry_run: true,
            args: vec!["env".to_string(), "python-3.9".to_string()],
        };
        assert!(execute(args).is_ok());
    }

    #[test]
    fn test_forward_no_args_shows_help() {
        let args = ForwardArgs {
            print_path: false,
            dry_run: false,
            args: vec![],
        };
        assert!(execute(args).is_ok());
    }
}
