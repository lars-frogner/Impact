use anyhow::Result;

#[cfg(feature = "cli")]
mod main {
    use super::*;
    use clap::{Parser, Subcommand};

    #[derive(Debug, Parser)]
    #[command(about = "The Impact math library", long_about = None)]
    struct Cli {
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Debug, Subcommand)]
    enum Command {
        #[cfg(feature = "benchmark")]
        /// Run a benchmarking target
        Benchmark {
            /// Benchmarking target to run
            #[arg(short, long, value_enum)]
            target: impact_math::benchmark::Target,

            /// Number of seconds to run the target for (it will always be run at least
            /// once)
            #[arg(short, long, default_value_t = 0.0)]
            duration: f64,

            /// Minimum number of seconds from the program is started until the target
            /// is run
            #[arg(long, default_value_t = 0.0)]
            delay: f64,
        },
        #[cfg(not(feature = "benchmark"))]
        /// Run a benchmarking target (requires the `benchmark` feature)
        Benchmark,
    }

    pub fn main() -> Result<()> {
        let cli = Cli::parse();

        match cli.command {
            #[cfg(feature = "benchmark")]
            Command::Benchmark {
                target,
                duration,
                delay,
            } => {
                impact_math::benchmark::benchmark(target, duration, delay);
                Ok(())
            }
            #[cfg(not(feature = "benchmark"))]
            Command::Benchmark => {
                anyhow::bail!(
                    "The `benchmark` subcommand requires the `benchmark` feature to be enabled."
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
