app [main!] {
    cli: platform "https://github.com/roc-lang/basic-cli/releases/download/0.19.0/Hj-J_zxz7V9YurCSTFcFdu6cQJie4guzsPMUi5kBYUk.tar.br",
}

import cli.Cmd
import cli.Stdout
import cli.Env

## Builds the [platform](https://www.roc-lang.org/platforms).
##
## run with: roc ./build.roc
main! : _ => Result {} _
main! = |_args|

    roc_cmd = Env.var!("ROC") |> Result.with_default("roc")

    debug_mode =
        when Env.var!("DEBUG") is
            Ok(str) if !(Str.is_empty(str)) -> Debug
            _ -> Release

    roc_version!(roc_cmd)?

    os_and_arch = get_os_and_arch!({})?

    cargo_build_platform!(debug_mode)?

    rust_target_folder = get_rust_target_folder!(debug_mode)?

    copy_platform_lib!(os_and_arch, rust_target_folder)?

    info!("Successfully built platform files!")?

    Ok({})

roc_version! : Str => Result {} _
roc_version! = |roc_cmd|
    info!("Checking provided roc; executing `${roc_cmd} version`:")?

    Cmd.exec!(roc_cmd, ["version"])
    |> Result.map_err(RocVersionCheckFailed)

get_os_and_arch! : {} => Result OSAndArch _
get_os_and_arch! = |{}|
    info!("Getting the native operating system and architecture ...")?

    convert_os_and_arch!(Env.platform!({}))

OSAndArch : [
    MacosArm64,
    MacosX64,
    LinuxArm64,
    LinuxX64,
    WindowsArm64,
    WindowsX64,
]

convert_os_and_arch! : _ => Result OSAndArch _
convert_os_and_arch! = |{ os, arch }|
    when (os, arch) is
        (MACOS, AARCH64) -> Ok(MacosArm64)
        (MACOS, X64) -> Ok(MacosX64)
        (LINUX, AARCH64) -> Ok(LinuxArm64)
        (LINUX, X64) -> Ok(LinuxX64)
        _ -> Err(UnsupportedNative(os, arch))

prebuilt_static_lib_file : OSAndArch -> Str
prebuilt_static_lib_file = |os_and_arch|
    when os_and_arch is
        MacosArm64 -> "macos-arm64.a"
        MacosX64 -> "macos-x64.a"
        LinuxArm64 -> "linux-arm64.a"
        LinuxX64 -> "linux-x64.a"
        WindowsArm64 -> "windows-arm64.lib"
        WindowsX64 -> "windows-x64.lib"

get_rust_target_folder! : [Debug, Release] => Result Str _
get_rust_target_folder! = |debug_mode|

    debug_or_release = if debug_mode == Debug then "debug" else "release"

    when Env.var!("CARGO_BUILD_TARGET") is
        Ok(target_env_var) ->
            if Str.is_empty(target_env_var) then
                Ok("target/${debug_or_release}/")
            else
                Ok("target/${target_env_var}/${debug_or_release}/")

        Err(e) ->
            info!("Failed to get env var CARGO_BUILD_TARGET with error ${Inspect.to_str(e)}. Assuming default CARGO_BUILD_TARGET (native)...")?

            Ok("target/${debug_or_release}/")

cargo_build_platform! : [Debug, Release] => Result {} _
cargo_build_platform! = |debug_mode|

    cargo_build_args! = |{}|
        when debug_mode is
            Debug ->
                info!("Building rust platform in debug mode...")?
                Ok(["build"])

            Release ->
                info!("Building rust platform ...")?
                Ok(["build", "--release"])

    args = cargo_build_args!({})?

    Cmd.exec!("cargo", args)
    |> Result.map_err(ErrBuildingPlatformBinaries)

copy_platform_lib! : OSAndArch, Str => Result {} _
copy_platform_lib! = |os_and_arch, rust_target_folder|

    platform_build_path = "${rust_target_folder}libroc_platform.a"

    platform_dest_path = "api/${prebuilt_static_lib_file(os_and_arch)}"

    info!("Moving the prebuilt binary from ${platform_build_path} to ${platform_dest_path} ...")?

    Cmd.exec!("cp", [platform_build_path, platform_dest_path])
    |> Result.map_err(ErrMovingPrebuiltLegacyBinary)

info! : Str => Result {} _
info! = |msg|
    Stdout.line!("\u(001b)[34mINFO:\u(001b)[0m ${msg}")
