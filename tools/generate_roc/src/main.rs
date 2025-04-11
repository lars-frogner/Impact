mod generate;

use anyhow::Result;
use clap::{Parser, Subcommand};
use generate::{GenerateOptions, RocGenerateOptions};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Generation of Roc code for the Impact game engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate Roc modules
    RocModules {
        /// Path to directory in which to put the modules
        #[arg(short, long)]
        target_dir: PathBuf,
        /// Print info messages
        #[arg(short, long)]
        verbose: bool,
        /// Overwrite any existing files in the target directory
        #[arg(long)]
        overwrite: bool,
        /// String to prepend to references to generated modules (e.g. `Generated.`)
        #[arg(long)]
        prefix: String,
        /// String to prepend to references to the `Core` module (e.g. `pf.`)
        #[arg(long)]
        core_prefix: String,
        /// Include tests
        #[arg(long)]
        tests: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::RocModules {
            target_dir,
            verbose,
            overwrite,
            prefix,
            core_prefix,
            tests,
        } => {
            let options = GenerateOptions { verbose, overwrite };
            let roc_options = RocGenerateOptions {
                module_prefix: prefix,
                core_prefix,
                include_roundtrip_test: tests,
            };
            generate::generate_roc(target_dir, &options, &roc_options)?;
        }
    }

    Ok(())
}
