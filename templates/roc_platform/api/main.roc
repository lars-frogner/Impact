platform "impact"
    requires {} {
        callbacks : {
            setup_scene! : {} => Result {} Str,
            handle_keyboard_event! : KeyboardEvent => Result {} Str,
        }
    }
    exposes [
        Command,
        Comp,
        Containers,
        Control,
        Entity,
        Input,
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
        handle_keyboard_event_extern!,
        command_roundtrip_extern!,
    ]

import Command.EngineCommand as EngineCommand
import Input.KeyboardEvent as KeyboardEvent exposing [KeyboardEvent]

setup_scene_extern! : I32 => Result {} Str
setup_scene_extern! = |_|
    callbacks.setup_scene!({})

handle_keyboard_event_extern! : List U8 => Result {} Str
handle_keyboard_event_extern! = |bytes|
    event = KeyboardEvent.from_bytes(bytes) |> map_err_to_str?
    callbacks.handle_keyboard_event!(event)

command_roundtrip_extern! : List U8 => Result (List U8) Str
command_roundtrip_extern! = |bytes|
    command = EngineCommand.from_bytes(bytes) |> map_err_to_str?
    Ok(EngineCommand.write_bytes([], command))

map_err_to_str = |result|
    result |> Result.map_err(|err| Inspect.to_str(err))
