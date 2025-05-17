use anyhow::Result;

#[cfg(feature = "cli")]
mod main {
    use super::*;
    use clap::{Parser, Subcommand};

    #[derive(Debug, Parser)]
    #[command(about = "The Impact ECS library", long_about = None)]
    struct Cli {
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Debug, Subcommand)]
    enum Command {
        #[cfg(feature = "profiling")]
        /// Run a profiling target
        Profile {
            /// Profiling target to run
            #[arg(short, long, value_enum)]
            target: impact_ecs::profiling::profile::Target,

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
            #[cfg(feature = "profiling")]
            Command::Profile {
                target,
                duration,
                delay,
            } => {
                impact_ecs::profiling::profile::profile(target, duration, delay);
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
