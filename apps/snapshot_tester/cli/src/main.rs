use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use dynamic_lib::DynamicLibrary;
use std::path::PathBuf;

dynamic_lib::define_lib! {
    name = AppLib,
    path_env_var = "APP_LIB_PATH",
    fallback_path = "./libapp";

    unsafe fn run_with_config_at_path(path_ptr: *const u8, path_len: usize) -> i32;
}

#[derive(Debug, Parser)]
#[command(about = "Snapshot tester for the Impact engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run all snapshot tests
    Run {
        /// Path to RON configuration file to use
        #[arg(short, long)]
        config_path: PathBuf,
    },
}

fn run(config_path: PathBuf) -> Result<()> {
    let config_path = config_path.to_string_lossy();
    let config_path_bytes = config_path.as_bytes();

    error_code_to_result(unsafe {
        AppLib::acquire()
            .run_with_config_at_path(config_path_bytes.as_ptr(), config_path_bytes.len())
    })
}

fn error_code_to_result(error_code: i32) -> Result<()> {
    if error_code == 0 {
        Ok(())
    } else {
        Err(anyhow!("App exited with error code {error_code}"))
    }
}

pub fn main() -> Result<()> {
    AppLib::load().context("Failed to load app library")?;

    let cli = Cli::parse();

    match cli.command {
        Command::Run { config_path } => run(config_path)?,
    }
    Ok(())
}
