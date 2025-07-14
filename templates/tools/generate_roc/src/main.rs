//! Any crate that wants to generate Roc code for some of its Rust types
//! can create its own copy of this tool, using `src` as is but adapting
//! `Cargo.toml` to depend on the crate in question. All registered Roc
//! types in all crates linked to this binary will be included.

// Make sure the target crate is linked in even if we don't use it.
pub use target_crate;

use anyhow::Result;
use clap::{Parser, Subcommand};
use roc_integration::generate::{
    self, CleanOptions, GenerateOptions, ListOptions, ListedRocTypeCategory, RocGenerateOptions,
};
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
        /// The categories of types to list (includes all categories if omitted)
        #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
        categories: Vec<ListedRocTypeCategory>,
        /// Show the type ID of each type
        #[arg(long)]
        show_type_ids: bool,
    },
    /// List associated constants and functions for types annotated with the `roc` attribute
    ListAssociatedItems {
        /// Specific types to list associated items for (includes all types if omitted)
        #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
        types: Vec<String>,
    },
    /// Generate Roc modules
    GenerateModules {
        /// Print info messages
        #[arg(short, long)]
        verbose: bool,
        /// Path to the Roc package directory in which to put the modules
        #[arg(short, long, value_name = "PATH")]
        target_dir: PathBuf,
        /// Name of the Roc package in which to put the modules (defaults to the directory name)
        #[arg(short, long, value_name = "NAME")]
        package_name: Option<String>,
        /// Specific modules to generate (for parent modules, all children are generated)
        #[arg(long, value_name = "MODULES", value_delimiter = ' ', num_args = 1..)]
        only: Vec<String>,
    },
    /// Remove generated Roc files
    Clean {
        /// Print info messages
        #[arg(short, long)]
        verbose: bool,
        /// Recurse into subdirectories
        #[arg(short, long)]
        recursive: bool,
        /// Path to the directory containing generated Roc files
        #[arg(short, long, value_name = "PATH")]
        target_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::ListTypes {
            categories,
            show_type_ids,
        } => {
            let options = ListOptions {
                categories,
                show_type_ids,
            };
            let component_type_ids = target_crate::gather_roc_type_ids_for_all_components();
            generate::list_types(options, &component_type_ids)?;
        }
        Command::ListAssociatedItems { types } => {
            generate::list_associated_items(types)?;
        }
        Command::GenerateModules {
            target_dir,
            package_name,
            only,
            verbose,
        } => {
            let options = GenerateOptions { verbose };
            let roc_options = RocGenerateOptions {
                package_name,
                only_modules: only,
            };
            let component_type_ids = target_crate::gather_roc_type_ids_for_all_components();
            generate::generate_roc(target_dir, options, roc_options, &component_type_ids)?;
        }
        Command::Clean {
            verbose,
            recursive,
            target_dir,
        } => {
            let options = CleanOptions { verbose, recursive };
            generate::clean_generated_roc(target_dir, options)?;
        }
    }

    Ok(())
}
