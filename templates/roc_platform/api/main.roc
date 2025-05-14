platform "impact"
    requires {} {
        setup_scene! : {} => Result {} Str,
    }
    exposes [
        Command,
        Comp,
        Containers,
        Control,
        Entity,
        Light,
        Material,
        Mesh,
        Model,
        Physics,
        Rendering,
        Scene,
        Stdout,
        Skybox,
        Voxel,
    ]
    packages {
        core: "../../../../roc_packages/core/main.roc",
    }
    imports []
    provides [
        setup_scene_extern!,
        command_roundtrip_extern!,
    ]

import Command.EngineCommand as EngineCommand

setup_scene_extern! : I32 => Result {} Str
setup_scene_extern! = |_|
    setup_scene!({})

command_roundtrip_extern! : List U8 => Result (List U8) Str
command_roundtrip_extern! = |bytes|
    command = EngineCommand.from_bytes(bytes) |> Result.map_err(|err| Inspect.to_str(err))?
    Ok(EngineCommand.write_bytes([], command))
