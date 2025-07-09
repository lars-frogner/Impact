use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use ffi_utils::define_ffi;
use std::path::PathBuf;

define_ffi! {
    name = AppFFI,
    lib_path_env = "APP_LIB_PATH",
    lib_path_default = "../../../lib/libapp",
    run_with_config_at_path => unsafe extern "C" fn(*const u8, usize) -> i32,
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

fn error_code_to_result(error_code: i32) -> Result<()> {
    if error_code == 0 {
        Ok(())
    } else {
        Err(anyhow!("App exited with error code {error_code}"))
    }
}

pub fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { config_path } => run(config_path)?,
    }
    Ok(())
}
