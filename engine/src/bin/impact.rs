use anyhow::Result;

#[cfg(feature = "cli")]
mod main {
    use super::*;
    use anyhow::bail;
    use clap::{Parser, Subcommand};
    use impact::{engine::EngineConfig, io::util::write_ron_file};
    use std::path::PathBuf;

    #[derive(Debug, Parser)]
    #[command(about = "The Impact game engine", long_about = None)]
    struct Cli {
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Debug, Subcommand)]
    enum Command {
        /// Generate the default RON configuration file
        GenerateConfig {
            /// Path where the file should be written
            #[arg(short, long)]
            output_path: PathBuf,
            /// Overwrite any existing file at the given path
            #[arg(short, long)]
            force_overwrite: bool,
        },
        #[cfg(feature = "profiling")]
        /// Run a profiling target
        Profile {
            /// Profiling target to run
            #[arg(short, long, value_enum)]
            target: impact::profiling::profile::Target,

            /// Number of seconds to run the target for (it will always be run at least
            /// once)
            #[arg(short, long, default_value_t = 0.0)]
            duration: f64,

            /// Minimum number of seconds from the program is started until the target
            /// is run
            #[arg(long, default_value_t = 0.0)]
            delay: f64,
        },
        #[cfg(not(feature = "profiling"))]
        /// Run a profiling target (requires the `profiling` feature)
        Profile,
    }

    pub fn main() -> Result<()> {
        let cli = Cli::parse();

        match cli.command {
            Command::GenerateConfig {
                output_path,
                force_overwrite,
            } => {
                if !force_overwrite && output_path.exists() {
                    bail!("File {} already exists", output_path.display());
                }
                let config = EngineConfig::default();
                write_ron_file(&config, output_path)
            }
            #[cfg(feature = "profiling")]
            Command::Profile {
                target,
                duration,
                delay,
            } => {
                impact::profiling::profile::profile(target, duration, delay);
                Ok(())
            }
            #[cfg(not(feature = "profiling"))]
            Command::Profile => {
                anyhow::bail!(
                    "The `profile` subcommand requires the `profiling` feature to be enabled."
                )
            }
        }
    }
}

#[cfg(not(feature = "cli"))]
mod main {
    use super::*;

    pub fn main() -> Result<()> {
        anyhow::bail!("This binary requires the `cli` feature to be enabled.")
    }
}

fn main() -> Result<()> {
    main::main()
}
