use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use ffi_helpers::define_ffi;
use std::path::PathBuf;

#[cfg(feature = "fuzzing")]
define_ffi! {
    name = AppFFI,
    lib_path_env = "APP_LIB_PATH",
    lib_path_default = "./libapp",
    run_with_config_at_path => unsafe extern "C" fn(*const u8, usize) -> i32,
    fuzz_test_command_roundtrip => unsafe extern "C" fn(usize, u64, u8) -> i32,
}
#[cfg(not(feature = "fuzzing"))]
define_ffi! {
    name = AppFFI,
    lib_path_env = "APP_LIB_PATH",
    lib_path_default = "./libapp",
    run_with_config_at_path => unsafe extern "C" fn(*const u8, usize) -> i32,
}

#[derive(Debug, Parser)]
#[command(about = "A basic Impact application", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the application
    Run {
        /// Path to RON configuration file to use
        #[arg(short, long)]
        config_path: PathBuf,
    },
    #[cfg(feature = "fuzzing")]
    /// Run a fuzz test
    Fuzz {
        /// Fuzz test to run
        #[arg(short, long)]
        test: FuzzTest,
        /// Number of test iterations to execute
        #[arg(short, long)]
        iterations: u64,
        /// Seed for randomly generated test inputs
        #[arg(short, long, default_value_t = 0)]
        seed: u64,
        /// Print status and progress messages
        #[arg(short, long)]
        verbose: bool,
    },
}

#[cfg(feature = "fuzzing")]
#[derive(Clone, Debug, clap::ValueEnum)]
enum FuzzTest {
    CommandRoundtrip,
}

fn run(config_path: PathBuf) -> Result<()> {
    let config_path = config_path.to_string_lossy();
    let config_path_bytes = config_path.as_bytes();

    AppFFI::call(
        |ffi| unsafe {
            error_code_to_result((ffi.run_with_config_at_path)(
                config_path_bytes.as_ptr(),
                config_path_bytes.len(),
            ))
        },
        |error| Err(anyhow!("{error:#}")),
    )
}

#[cfg(feature = "fuzzing")]
fn fuzz(test: FuzzTest, iterations: u64, seed: u64, verbose: bool) -> Result<()> {
    match test {
        FuzzTest::CommandRoundtrip => AppFFI::call(
            |ffi| unsafe {
                error_code_to_result((ffi.fuzz_test_command_roundtrip)(
                    iterations as usize,
                    seed,
                    if verbose { 1 } else { 0 },
                ))
            },
            |error| Err(anyhow!("{error:#}")),
        ),
    }
}

fn error_code_to_result(error_code: i32) -> Result<()> {
    if error_code == 0 {
        Ok(())
    } else {
        Err(anyhow!("Exited with error code {error_code}"))
    }
}

pub fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { config_path } => run(config_path)?,
        #[cfg(feature = "fuzzing")]
        Command::Fuzz {
            test,
            iterations,
            seed,
            verbose,
        } => fuzz(test, iterations, seed, verbose)?,
    }
    Ok(())
}
