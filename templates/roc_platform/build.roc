app [main!] {
    cli: platform "https://github.com/roc-lang/basic-cli/releases/download/0.19.0/Hj-J_zxz7V9YurCSTFcFdu6cQJie4guzsPMUi5kBYUk.tar.br",
}

import cli.Cmd
import cli.Env
import cli.Stdout

main! : _ => Result {} _
main! = |_args|
    platform_dir =
        when Env.var!("PLATFORM_DIR") is
            Ok(str) if !(Str.is_empty(str)) -> str
            _ -> "."

    linker =
        when Env.var!("MOLD") is
            Ok(str) if !(Str.is_empty(str)) -> Mold
            _ -> Ld

    debug_mode =
        when Env.var!("DEBUG") is
            Ok(str) if !(Str.is_empty(str)) -> Debug
            _ -> Release

    cargo_build_platform!(platform_dir, linker, debug_mode)?

    rust_target_folder = get_rust_target_folder!(debug_mode)

    copy_platform_lib!(platform_dir, rust_target_folder)?

    Ok({})

get_rust_target_folder! : [Debug, Release] => Str
get_rust_target_folder! = |debug_mode|
    debug_or_release = if debug_mode == Debug then "debug" else "release"

    when Env.var!("CARGO_BUILD_TARGET") is
        Ok(target_env_var) ->
            if Str.is_empty(target_env_var) then
                "target/${debug_or_release}/"
            else
                "target/${target_env_var}/${debug_or_release}/"

        Err(_) ->
            "target/${debug_or_release}/"

cargo_build_platform! : Str, [Ld, Mold], [Debug, Release] => Result {} _
cargo_build_platform! = |platform_dir, linker, debug_mode|
    Stdout.line!("Building platform crate with options: ${Inspect.to_str(linker)}, ${Inspect.to_str(debug_mode)}")?

    base_args = ["--manifest-path", "${platform_dir}/Cargo.toml"]

    debug_args =
        when debug_mode is
            Debug -> []
            Release -> ["--release"]

    linker_env_vars =
        when linker is
            Ld -> []
            Mold -> ["RUSTFLAGS=-C link-arg=-fuse-ld=mold"]

    Cmd.exec!(
        "env",
        linker_env_vars
        |> List.concat(["cargo", "build"])
        |> List.concat(base_args)
        |> List.concat(debug_args),
    )
    |> Result.map_err(ErrBuildingPlatformLibrary)

copy_platform_lib! : Str, Str => Result {} _
copy_platform_lib! = |platform_dir, rust_target_folder|
    Stdout.line!("Copying platform library to platform lib folder")?

    platform_build_path = "${platform_dir}/${rust_target_folder}libroc_platform.a"
    platform_dest_path = "${platform_dir}/lib/"

    Cmd.exec!("cp", [platform_build_path, platform_dest_path])
    |> Result.map_err(ErrCopyingPlatformLibrary)
