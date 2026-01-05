//! Hot reloading of the script library.

use super::ScriptLib;
use anyhow::Result;
use dynamic_lib::{LoadableLibrary, hot_reloading::LibraryReloader};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub type ScriptReloader = LibraryReloader<ScriptLib>;

pub fn create_script_reloader() -> Result<ScriptReloader> {
    let source_dir_path = obtain_script_source_dir_path()?;

    let build_output_lib_path = obtain_build_output_lib_path_for_reloader()?;

    let build_command =
        create_build_command_for_reloader(&source_dir_path, &build_output_lib_path)?;

    let script_reloader =
        ScriptReloader::new(source_dir_path, build_command, build_output_lib_path)?;

    Ok(script_reloader)
}

fn obtain_script_source_dir_path() -> Result<PathBuf> {
    dynamic_lib::resolve_path_from_env_with_fallback("SCRIPT_SOURCE_DIR_PATH", "../../scripts")
        .map_err(Into::into)
}

fn obtain_build_output_lib_path_for_reloader() -> Result<PathBuf> {
    let script_lib_path = ScriptLib::resolved_path()?;

    let output_dir_path = script_lib_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("hot_reload");

    let output_lib_path = output_dir_path.join("libscript");

    Ok(output_lib_path)
}

fn create_build_command_for_reloader(
    source_dir_path: &Path,
    build_output_lib_path: &Path,
) -> Result<Command> {
    let builder_dir_path = dynamic_lib::resolve_path_from_env_with_fallback(
        "SCRIPT_BUILDER_DIR_PATH",
        source_dir_path.parent().unwrap(),
    )?;

    let mut command = Command::new("./build_script");

    command.current_dir(builder_dir_path);

    command.env("OUTPUT_DIR", build_output_lib_path.parent().unwrap());

    command.env("SCRIPT_ONLY", "1");
    command.env("ROC_DEBUG", "1");

    Ok(command)
}
