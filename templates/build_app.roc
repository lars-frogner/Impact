app [main!] {
    cli: platform "https://github.com/roc-lang/basic-cli/releases/download/0.19.0/Hj-J_zxz7V9YurCSTFcFdu6cQJie4guzsPMUi5kBYUk.tar.br",
}

import cli.Cmd
import cli.Env
import cli.Stdout

main! : _ => Result {} _
main! = |_args|
    app_dir =
        when Env.var!("APP_DIR") is
            Ok(str) if !(Str.is_empty(str)) -> str
            _ -> "."

    platform_dir =
        when Env.var!("PLATFORM_DIR") is
            Ok(str) if !(Str.is_empty(str)) -> str
            _ -> "${app_dir}/roc_platform"

    backend =
        when Env.var!("CRANELIFT") is
            Ok(str) if !(Str.is_empty(str)) -> Cranelift
            _ -> LLVM

    linker =
        when Env.var!("MOLD") is
            Ok(str) if !(Str.is_empty(str)) -> Mold
            _ -> Ld

    debug_mode =
        when Env.var!("DEBUG") is
            Ok(str) if !(Str.is_empty(str)) -> Debug
            _ -> Release

    asan_mode =
        when Env.var!("ASAN") is
            Ok(str) if !(Str.is_empty(str)) -> AddressSanitizer
            _ -> NoAddressSanitizer

    fuzzing_mode =
        when Env.var!("FUZZING") is
            Ok(str) if !(Str.is_empty(str)) -> Fuzzing
            _ -> NoFuzzing

    os_and_arch = get_os_and_arch!({})?

    build_platform!(platform_dir)?

    cargo_build_app!(app_dir, backend, linker, debug_mode, asan_mode, fuzzing_mode, os_and_arch)?

    copy_app_lib!(app_dir, debug_mode, os_and_arch)?

    roc_build_script!(app_dir, debug_mode)?

    link_script_with_platform!(platform_dir, app_dir)?

    Ok({})

get_os_and_arch! : {} => Result OSAndArch _
get_os_and_arch! = |{}|
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

get_target_triple : OSAndArch -> Str
get_target_triple = |os_and_arch|
    when os_and_arch is
        MacosX64 -> "x86_64-apple-darwin"
        MacosArm64 -> "aarch64-apple-darwin"
        LinuxX64 -> "x86_64-unknown-linux-gnu"
        LinuxArm64 -> "aarch64-unknown-linux-gnu"
        WindowsX64 -> "x86_64-pc-windows-msvc"
        WindowsArm64 -> "aarch64-pc-windows-msvc"

build_platform! : Str => Result {} _
build_platform! = |platform_dir|
    Cmd.exec!("env", ["PLATFORM_DIR=${platform_dir}", "roc", "${platform_dir}/build.roc"])
    |> Result.map_err(ErrBuildingPlatformLibrary)

cargo_build_app! : Str, [LLVM, Cranelift], [Ld, Mold], [Debug, Release], [AddressSanitizer, NoAddressSanitizer], [Fuzzing, NoFuzzing], OSAndArch => Result {} _
cargo_build_app! = |app_dir, backend, linker, debug_mode, asan_mode, fuzzing_mode, os_and_arch|
    Stdout.line!("Building application crate with options: ${Inspect.to_str(backend)}, ${Inspect.to_str(linker)}, ${Inspect.to_str(debug_mode)}, ${Inspect.to_str(asan_mode)}, ${Inspect.to_str(fuzzing_mode)}")?

    target_triple = get_target_triple(os_and_arch)

    base_args = ["--manifest-path", "${app_dir}/Cargo.toml", "--target", target_triple]

    nightly_arg =
        when (backend, asan_mode) is
            (Cranelift, _) | (_, AddressSanitizer) -> ["+nightly"]
            _ -> []

    backend_args =
        when backend is
            LLVM -> []
            Cranelift -> ["-Zcodegen-backend"]

    debug_args =
        when debug_mode is
            Debug -> []
            Release -> ["--release"]

    fuzzing_args =
        when fuzzing_mode is
            NoFuzzing -> []
            Fuzzing -> ["--features", "fuzzing"]

    backend_env_vars =
        when backend is
            LLVM -> []
            Cranelift -> ["CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift"]

    linker_env_vars =
        when linker is
            Ld -> []
            Mold -> ["RUSTFLAGS=-C link-arg=-fuse-ld=mold"]

    asan_env_vars =
        when asan_mode is
            NoAddressSanitizer -> []
            AddressSanitizer ->
                [
                    "RUSTFLAGS=-C debuginfo=2 -C debug-assertions -C overflow-checks=yes -Z sanitizer=address -C link-arg=-lasan",
                ]

    Cmd.exec!(
        "env",
        backend_env_vars
        |> List.concat(linker_env_vars)
        |> List.concat(asan_env_vars)
        |> List.append("cargo")
        |> List.concat(nightly_arg)
        |> List.append("build")
        |> List.concat(base_args)
        |> List.concat(backend_args)
        |> List.concat(debug_args)
        |> List.concat(fuzzing_args),
    )
    |> Result.map_err(ErrBuildingAppLibrary)

copy_app_lib! : Str, [Debug, Release], OSAndArch => Result {} _
copy_app_lib! = |app_dir, debug_mode, os_and_arch|
    Stdout.line!("Copying application library to application lib folder")?

    crate_name = find_crate_name!(app_dir)?
    lib_extension = lib_file_extension(os_and_arch)
    target_triple = get_target_triple(os_and_arch)
    rust_target_folder = get_rust_target_folder!(debug_mode, target_triple)
    app_build_path = "${app_dir}/${rust_target_folder}lib${crate_name}.${lib_extension}"
    app_dest_path = "${app_dir}/lib/libapp"

    Cmd.exec!("cp", [app_build_path, app_dest_path])
    |> Result.map_err(ErrCopyingAppLibrary)

find_crate_name! : Str => Result Str _
find_crate_name! = |app_dir|
    when Env.var!("CRATE_NAME") is
        Ok(str) if !(Str.is_empty(str)) -> Ok(str)
        _ -> find_crate_name_from_app_dir!(app_dir)

find_crate_name_from_app_dir! : Str => Result Str _
find_crate_name_from_app_dir! = |app_dir|
    Ok(app_dir |> resolve_path!? |> Str.trim |> Str.split_last("/")? |> .after)

resolve_path! : Str => Result Str _
resolve_path! = |path|
    output =
        Cmd.new("realpath")
        |> Cmd.arg(path)
        |> Cmd.output!
    when output.status is
        Ok(_) -> Ok(output.stdout |> Str.from_utf8?)
        Err(err) -> Err(ErrResolvingPath(err))

lib_file_extension : OSAndArch -> Str
lib_file_extension = |os_and_arch|
    when os_and_arch is
        MacosX64 | MacosArm64 -> "dylib"
        LinuxArm64 | LinuxX64 -> "so"
        WindowsX64 | WindowsArm64 -> "dll"

get_rust_target_folder! : [Debug, Release], Str => Str
get_rust_target_folder! = |debug_mode, target_triple|
    debug_or_release = if debug_mode == Debug then "debug" else "release"

    when Env.var!("CARGO_BUILD_TARGET") is
        Ok(target_env_var) ->
            if Str.is_empty(target_env_var) then
                "target/${target_triple}/${debug_or_release}/"
            else
                "target/${target_env_var}/${debug_or_release}/"

        Err(_) ->
            "target/${target_triple}/${debug_or_release}/"

roc_build_script! : Str, [Debug, Release] => Result {} _
roc_build_script! = |app_dir, debug_mode|
    Stdout.line!("Building Roc script")?

    base_args = ["build", "--no-link", "${app_dir}/scripts/main.roc", "--output", "${app_dir}/lib/script.o"]
    opt_args =
        when debug_mode is
            Debug -> []
            Release -> ["--optimize"]

    result = Cmd.exec!("roc", List.concat(base_args, opt_args))

    when result is
        Ok(_) -> Ok({})
        Err(CmdStatusErr(Other(msg))) ->
            if Str.ends_with(msg, "exit code: 2") then
                Ok({}) # Build warnings
            else
                Err(CmdStatusErr(Other(msg)))

        err -> err

link_script_with_platform! : Str, Str => Result {} _
link_script_with_platform! = |platform_dir, app_dir|
    Stdout.line!("Linking Roc script with platform library")?
    Cmd.exec!(
        "cc",
        [
            "-shared",
            "-o",
            "${app_dir}/lib/libscript",
            "${app_dir}/lib/script.o",
            "${platform_dir}/lib/libroc_platform.a",
            "-lm", # Some Roc builtins require `libm`
        ],
    )
    |> Result.map_err(ErrLinkingScriptWithPlatform)
