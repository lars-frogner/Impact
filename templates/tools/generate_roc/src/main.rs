//! Any crate that wants to generate Roc code for some of its Rust types
//! can create its own copy of this tool, using `src` as is but adapting
//! `Cargo.toml` to depend on the crate in question. All registered Roc
//! types in all crates linked to this binary will be included.

// Make sure the target crate is linked in even if we don't use it.
pub use target_crate;

use anyhow::Result;
use clap::{Parser, Subcommand};
use roc_codegen::generate::{self, GenerateOptions, ListOptions, ListedRocTypeCategory};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Generation of Roc code for the Impact game engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List types annotated with the `roc` attribute
    ListTypes {
        /// The categories of types to list
        #[arg(short, long, value_delimiter=' ', num_args=1.., required = true)]
        categories: Vec<ListedRocTypeCategory>,
    },
    /// List associated constants and functions for types annotated with the `roc` attribute
    ListAssociatedItems {
        /// Specific types to list associated items for (includes all types if omitted)
        #[arg(short, long, value_delimiter = ' ')]
        types: Vec<String>,
    },
    /// Generate Roc modules
    GenerateModules {
        /// Path to the Roc package in which to put the modules
        #[arg(short, long)]
        package_root: PathBuf,
        /// Print info messages
        #[arg(short, long)]
        verbose: bool,
        /// Overwrite any existing files in the target package
        #[arg(long)]
        overwrite: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::ListTypes { categories } => {
            let options = ListOptions {
                categories: categories.into_iter().collect(),
            };
            let component_type_ids = target_crate::gather_roc_type_ids_for_all_components();
            generate::list_types(&options, &component_type_ids)?;
        }
        Command::ListAssociatedItems { types } => {
            generate::list_associated_items(types)?;
        }
        Command::GenerateModules {
            package_root,
            verbose,
            overwrite,
        } => {
            let options = GenerateOptions { verbose, overwrite };
            let component_type_ids = target_crate::gather_roc_type_ids_for_all_components();
            generate::generate_roc(package_root, &options, &component_type_ids)?;
        }
    }

    Ok(())
}
