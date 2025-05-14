platform "impact"
    requires {} {
        setup_scene! : {} => Result {} Str,
    }
    exposes [Stdout, Entity, Scene]
    packages {}
    imports []
    provides [
        setup_scene_extern!,
        command_roundtrip_extern!,
    ]

import Command.EngineCommand as EngineCommand
import Stdout

setup_scene_extern! : I32 => Result {} Str
setup_scene_extern! = |_|
    setup_scene!({})

command_roundtrip_extern! : List U8 => Result (List U8) Str
command_roundtrip_extern! = |bytes|
    command = EngineCommand.from_bytes(bytes) |> Result.map_err(|err| Inspect.to_str(err))?
    Ok(EngineCommand.write_bytes([], command))
