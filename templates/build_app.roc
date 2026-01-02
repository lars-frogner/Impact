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

    target =
        when Env.var!("SCRIPT_ONLY") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Script
            _ -> All

    roc_debug_mode =
        when Env.var!("ROC_DEBUG") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Debug
            _ -> Release

    output_dir =
        when Env.var!("OUTPUT_DIR") is
            Ok(str) if !(Str.is_empty(str)) -> str
            _ -> "dist"

    when target is
        All -> build_all!(app_dir, platform_dir, output_dir, roc_debug_mode)
        Script -> build_script_only!(app_dir, platform_dir, output_dir, roc_debug_mode)

build_all! : Str, Str, Str, [Debug, Release] => Result {} _
build_all! = |app_dir, platform_dir, output_dir, roc_debug_mode|
    backend =
        when Env.var!("CRANELIFT") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Cranelift
            _ -> LLVM

    linker =
        when Env.var!("MOLD") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Mold
            _ -> Ld

    rust_debug_mode =
        when Env.var!("RUST_DEBUG") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Debug
            _ -> Release

    asan_mode =
        when Env.var!("ASAN") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> AddressSanitizer
            _ -> NoAddressSanitizer

    profiling_mode =
        when Env.var!("PROFILING") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Profiling
            _ -> NoProfiling

    tracy_mode =
        when Env.var!("TRACY") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Tracy
            _ -> NoTracy

    valgrind_mode =
        when Env.var!("VALGRIND") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Valgrind
            _ -> NoValgrind

    fuzzing_mode =
        when Env.var!("FUZZING") is
            Ok(str) if !(Str.is_empty(str)) and str != "0" -> Fuzzing
            _ -> NoFuzzing

    os_and_arch = get_os_and_arch!({})?

    build_platform!(platform_dir)?

    cargo_build_app!(app_dir, backend, linker, rust_debug_mode, asan_mode, profiling_mode, tracy_mode, valgrind_mode, fuzzing_mode, os_and_arch)?

    copy_app_lib!(app_dir, rust_debug_mode, os_and_arch)?

    roc_build_script!(app_dir, roc_debug_mode)?

    link_script_with_platform!(platform_dir, app_dir)?

    cargo_build_cli!(app_dir, rust_debug_mode, valgrind_mode, fuzzing_mode, os_and_arch)?

    crate_name = find_crate_name!(app_dir)?

    create_distribution!(app_dir, output_dir, crate_name, rust_debug_mode, os_and_arch)?

    Stdout.line!("Build complete")?

    Ok({})

build_script_only! : Str, Str, Str, [Debug, Release] => Result {} _
build_script_only! = |app_dir, platform_dir, output_dir, roc_debug_mode|
    roc_build_script!(app_dir, roc_debug_mode)?

    link_script_with_platform!(platform_dir, app_dir)?

    distribute_script!(app_dir, output_dir)?

    Stdout.line!("Script build complete")?

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

cargo_build_app! : Str, [LLVM, Cranelift], [Ld, Mold], [Debug, Release], [AddressSanitizer, NoAddressSanitizer], [Profiling, NoProfiling], [Tracy, NoTracy], [Valgrind, NoValgrind], [Fuzzing, NoFuzzing], OSAndArch => Result {} _
cargo_build_app! = |app_dir, backend, linker, debug_mode, asan_mode, profiling_mode, tracy_mode, valgrind_mode, fuzzing_mode, os_and_arch|
    Stdout.line!("Building application crate with options: ${Inspect.to_str(backend)}, ${Inspect.to_str(linker)}, ${Inspect.to_str(debug_mode)}, ${Inspect.to_str(asan_mode)}, ${Inspect.to_str(profiling_mode)}, ${Inspect.to_str(tracy_mode)}, ${Inspect.to_str(valgrind_mode)}, ${Inspect.to_str(fuzzing_mode)}")?

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

    tracy_args =
        when tracy_mode is
            NoTracy -> []
            Tracy -> ["--features", "tracy"]

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

    profiling_env_vars =
        when profiling_mode is
            NoProfiling -> []
            Profiling -> ["RUSTFLAGS=-C debuginfo=2 -C force-frame-pointers=yes -C strip=none"]

    valgrind_env_vars =
        when valgrind_mode is
            NoValgrind -> []
            Valgrind ->
                [
                    # Valgrind doesn't handle AVX512
                    "RUSTFLAGS=-C target-cpu=x86-64",
                ]

    Cmd.exec!(
        "env",
        backend_env_vars
        |> List.concat(linker_env_vars)
        |> List.concat(asan_env_vars)
        |> List.concat(profiling_env_vars)
        |> List.concat(valgrind_env_vars)
        |> List.append("cargo")
        |> List.concat(nightly_arg)
        |> List.append("build")
        |> List.concat(base_args)
        |> List.concat(backend_args)
        |> List.concat(debug_args)
        |> List.concat(tracy_args)
        |> List.concat(fuzzing_args),
    )
    |> Result.map_err(ErrBuildingAppLibrary)

copy_app_lib! : Str, [Debug, Release], OSAndArch => Result {} _
copy_app_lib! = |app_dir, debug_mode, os_and_arch|
    Stdout.line!("Copying application library to application lib folder")?

    Cmd.exec!("mkdir", ["-p", "${app_dir}/lib"])
    |> Result.map_err(ErrCreatingLibDirectory)?

    crate_name = find_crate_name!(app_dir)?
    lib_extension = lib_file_extension(os_and_arch)
    target_triple = get_target_triple(os_and_arch)
    rust_target_dir = get_rust_target_dir!(app_dir, debug_mode, target_triple)
    app_build_path = "${rust_target_dir}lib${crate_name}.${lib_extension}"
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

get_rust_target_dir! : Str, [Debug, Release], Str => Str
get_rust_target_dir! = |app_dir, debug_mode, target_triple|
    target_root =
        when Env.var!("CARGO_TARGET_DIR") is
            Ok(str) if !(Str.is_empty(str)) -> str
            _ -> "${app_dir}/target"

    debug_or_release = if debug_mode == Debug then "debug" else "release"

    when Env.var!("CARGO_BUILD_TARGET") is
        Ok(target_env_var) ->
            if Str.is_empty(target_env_var) then
                "${target_root}/${target_triple}/${debug_or_release}/"
            else
                "${target_root}/${target_env_var}/${debug_or_release}/"

        Err(_) ->
            "${target_root}/${target_triple}/${debug_or_release}/"

roc_build_script! : Str, [Debug, Release] => Result {} _
roc_build_script! = |app_dir, debug_mode|
    Stdout.line!("Building Roc script with options: ${Inspect.to_str(debug_mode)}")?

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

cargo_build_cli! : Str, [Debug, Release], [Valgrind, NoValgrind], [Fuzzing, NoFuzzing], OSAndArch => Result {} _
cargo_build_cli! = |app_dir, debug_mode, valgrind_mode, fuzzing_mode, os_and_arch|
    Stdout.line!("Building CLI binary with options: ${Inspect.to_str(debug_mode)}, ${Inspect.to_str(fuzzing_mode)}")?

    target_triple = get_target_triple(os_and_arch)

    base_args = ["--manifest-path", "${app_dir}/cli/Cargo.toml", "--target", target_triple]

    debug_args =
        when debug_mode is
            Debug -> []
            Release -> ["--release"]

    fuzzing_args =
        when fuzzing_mode is
            NoFuzzing -> []
            Fuzzing -> ["--features", "fuzzing"]

    valgrind_env_vars =
        when valgrind_mode is
            NoValgrind -> []
            Valgrind ->
                [
                    "RUSTFLAGS=-C target-cpu=x86-64",
                ]

    Cmd.exec!(
        "env",
        valgrind_env_vars
        |> List.concat(["cargo", "build"])
        |> List.concat(base_args)
        |> List.concat(debug_args)
        |> List.concat(fuzzing_args),
    )
    |> Result.map_err(ErrBuildingCLI)

create_distribution! : Str, Str, Str, [Debug, Release], OSAndArch => Result {} _
create_distribution! = |app_dir, output_dir, crate_name, debug_mode, os_and_arch|
    Stdout.line!("Creating distribution at ${output_dir}/")?
    distribute_app!(app_dir, output_dir)?
    distribute_script!(app_dir, output_dir)?
    distribute_binary!(app_dir, output_dir, crate_name, debug_mode, os_and_arch)

distribute_app! : Str, Str => Result {} _
distribute_app! = |app_dir, output_dir|
    Cmd.exec!("mkdir", ["-p", "${output_dir}"])
    |> Result.map_err(ErrCreatingDistDirectory)?

    Cmd.exec!("cp", ["${app_dir}/lib/libapp", "${output_dir}/"])
    |> Result.map_err(ErrCopyingToDistDirectory)

distribute_script! : Str, Str => Result {} _
distribute_script! = |app_dir, output_dir|
    Cmd.exec!("cp", ["${app_dir}/lib/libscript", "${output_dir}/"])
    |> Result.map_err(ErrCopyingToDistDirectory)

distribute_binary! : Str, Str, Str, [Debug, Release], OSAndArch => Result {} _
distribute_binary! = |app_dir, output_dir, crate_name, debug_mode, os_and_arch|
    cli_target_dir = get_cli_target_dir!(app_dir, debug_mode, os_and_arch)
    cli_binary_name = get_cli_binary_name(crate_name, os_and_arch)
    cli_dest_name = get_cli_dest_name(crate_name, os_and_arch)

    Cmd.exec!("cp", ["${cli_target_dir}${cli_binary_name}", "${output_dir}/${cli_dest_name}"])
    |> Result.map_err(ErrCopyingToDistDirectory)

get_cli_target_dir! : Str, [Debug, Release], OSAndArch => Str
get_cli_target_dir! = |app_dir, debug_mode, os_and_arch|
    target_root =
        when Env.var!("CARGO_TARGET_DIR") is
            Ok(str) if !(Str.is_empty(str)) -> str
            _ -> "${app_dir}/cli/target"

    target_triple = get_target_triple(os_and_arch)
    debug_or_release = if debug_mode == Debug then "debug" else "release"

    when Env.var!("CARGO_BUILD_TARGET") is
        Ok(target_env_var) ->
            if Str.is_empty(target_env_var) then
                "${target_root}/${target_triple}/${debug_or_release}/"
            else
                "${target_root}/${target_env_var}/${debug_or_release}/"

        Err(_) ->
            "${target_root}/${target_triple}/${debug_or_release}/"

get_cli_binary_name : Str, OSAndArch -> Str
get_cli_binary_name = |crate_name, os_and_arch|
    when os_and_arch is
        WindowsX64 | WindowsArm64 -> "${crate_name}_cli.exe"
        _ -> "${crate_name}_cli"

get_cli_dest_name : Str, OSAndArch -> Str
get_cli_dest_name = |crate_name, os_and_arch|
    when os_and_arch is
        WindowsX64 | WindowsArm64 -> "${crate_name}.exe"
        _ -> crate_name
