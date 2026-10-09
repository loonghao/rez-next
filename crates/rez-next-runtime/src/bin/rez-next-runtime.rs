//! Thin JSON interface to the public installation-plan facade.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use rez_next_runtime::InstallationPlan;

#[derive(Parser)]
#[command(version, about = "Read-only Rez runtime integration operations")]
struct Cli {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// Derive canonical package and variant paths using Core's strict solver.
    InstallationPlan {
        /// The unchanged package definition to parse through Rez Core.
        #[arg(long)]
        definition: PathBuf,
        /// Explicit Rez platform: windows, linux or osx.
        #[arg(long)]
        platform: String,
        /// Explicit Rez architecture version, such as x86_64 or arm_64.
        #[arg(long)]
        arch: String,
        /// Canonical local repository supplying real runtime dependencies.
        #[arg(long = "repository")]
        repositories: Vec<PathBuf>,
        /// Emit the stable machine-readable installation plan.
        #[arg(long, required = true)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let Operation::InstallationPlan {
        definition,
        platform,
        arch,
        repositories,
        json: _,
    } = cli.command;
    let plan = InstallationPlan::from_definition_with_repositories(
        definition,
        &platform,
        &arch,
        &repositories,
    )
    .await?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &plan)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
