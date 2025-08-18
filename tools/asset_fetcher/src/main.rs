mod asset;
mod fetch;
mod providers;

use anyhow::Result;
use asset::AssetList;
use clap::{Parser, Subcommand};
use impact_containers::HashSet;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(about = "Retrieval of assets for use in the Impact engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Ensure all assets in an asset list are present in a target directory
    Sync {
        /// Path to the file listing the desired assets
        #[arg(short, long, value_name = "PATH")]
        asset_list: PathBuf,
        /// Path to the asset directory that should be synchronized
        #[arg(short, long, value_name = "PATH")]
        target_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Sync {
            asset_list: asset_list_path,
            target_dir,
        } => {
            let asset_list: AssetList = impact_io::parse_ron_file(asset_list_path)?;

            // Ensure target directory exists
            std::fs::create_dir_all(&target_dir)?;

            let present_assets = determine_present_assets(&target_dir)?;

            let mut failed_assets = Vec::new();

            for asset in asset_list
                .into_iter()
                .filter(|asset| !present_assets.contains(&asset.name))
            {
                if let Err(e) = fetch::fetch_asset(&asset, &target_dir) {
                    eprintln!("Error fetching asset '{}': {}", asset.name, e);
                    failed_assets.push(asset.name);
                }
            }

            if !failed_assets.is_empty() {
                eprintln!(
                    "Failed to fetch {} asset(s): {}",
                    failed_assets.len(),
                    failed_assets.join(", ")
                );
            }
        }
    }

    Ok(())
}

fn determine_present_assets(target_dir: &Path) -> Result<HashSet<String>> {
    let mut present_assets = HashSet::default();

    if !target_dir.exists() {
        return Ok(present_assets);
    }

    for entry in fs::read_dir(target_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str() {
                present_assets.insert(name.to_string());
            }
    }

    Ok(present_assets)
}
