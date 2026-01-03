use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use dynamic_lib::DynamicLibrary;
use std::path::PathBuf;

#[cfg(feature = "fuzzing")]
dynamic_lib::define_lib! {
    name = AppLib,
    path_env_var = "APP_LIB_PATH",
    fallback_path = "./libapp";

    unsafe fn run_with_config_at_path(path_ptr: *const u8, path_len: usize) -> i32;
    unsafe fn fuzz_test_command_roundtrip(iterations: usize, seed: u64, verbose: u8) -> i32;
}
#[cfg(not(feature = "fuzzing"))]
dynamic_lib::define_lib! {
    name = AppLib,
    path_env_var = "APP_LIB_PATH",
    fallback_path = "./libapp";

    unsafe fn run_with_config_at_path(path_ptr: *const u8, path_len: usize) -> i32;
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

    error_code_to_result(unsafe {
        AppLib::acquire()
            .run_with_config_at_path(config_path_bytes.as_ptr(), config_path_bytes.len())
    })
}

#[cfg(feature = "fuzzing")]
fn fuzz(test: FuzzTest, iterations: u64, seed: u64, verbose: bool) -> Result<()> {
    match test {
        FuzzTest::CommandRoundtrip => error_code_to_result(unsafe {
            AppLib::acquire().fuzz_test_command_roundtrip(
                iterations as usize,
                seed,
                if verbose { 1 } else { 0 },
            )
        }),
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
    AppLib::load().context("Failed to load app library")?;

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
