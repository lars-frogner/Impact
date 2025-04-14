//! Any crate that wants to generate Roc code for some of its Rust types
//! can create its own copy of this tool, using `main.rs` as is but adapting
//! `Cargo.toml` to depend on the crate in question and modifying `generate.rs`
//! to re-export all contents of the `generate` module from the instance of the
//! `roc_codegen` crate it depends one. All registered Roc types in all crates
//! linked to this binary will be included.

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
        /// String to prepend to imports from generated modules (e.g. `Generated.`)
        #[arg(long, default_value = "")]
        import_prefix: String,
        /// Name to use for the platform package in imports
        #[arg(long, default_value = "pf")]
        platform_package_name: String,
        /// Name to use for the `packages/core` package in imports
        #[arg(long, default_value = "core")]
        core_package_name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::RocModules {
            target_dir,
            verbose,
            overwrite,
            import_prefix,
            platform_package_name,
            core_package_name,
        } => {
            let options = GenerateOptions { verbose, overwrite };
            let roc_options = RocGenerateOptions {
                import_prefix,
                platform_package_name,
                core_package_name,
            };
            let component_type_ids = generate::gather_roc_type_ids_for_all_components();
            generate::generate_roc(target_dir, &options, &roc_options, &component_type_ids)?;
        }
    }

    Ok(())
}
