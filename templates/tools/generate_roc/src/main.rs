//! Any crate that wants to generate Roc code for some of its Rust types
//! can create its own copy of this tool, using `src` as is but adapting
//! `Cargo.toml` to depend on the crate in question. All registered Roc
//! types in all crates linked to this binary will be included.

// Make sure the target crate is linked in even if we don't use it.
pub use target_crate;

use anyhow::Result;
use clap::{Parser, Subcommand};
use roc_codegen::generate::{
    self, GenerateOptions, ListOptions, ListedRocTypeCategory, RocGenerateOptions,
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
        #[arg(short, long, value_delimiter=' ', num_args=1.., required = true)]
        categories: Vec<ListedRocTypeCategory>,
    },
    /// List constructors in impl blocks annotated with the `roc` attribute
    ListConstructors,
    /// Generate Roc modules
    GenerateModules {
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
        Command::ListTypes { categories } => {
            let options = ListOptions {
                categories: categories.into_iter().collect(),
            };
            let component_type_ids = target_crate::gather_roc_type_ids_for_all_components();
            generate::list_types(&options, &component_type_ids)?;
        }
        Command::ListConstructors => {
            generate::list_constructors()?;
        }
        Command::GenerateModules {
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
            let component_type_ids = target_crate::gather_roc_type_ids_for_all_components();
            generate::generate_roc(target_dir, &options, &roc_options, &component_type_ids)?;
        }
    }

    Ok(())
}
