platform "impact"
    requires {} {
        callbacks : {
            setup_scene! : {} => Result {} Str,
            handle_keyboard_event! : KeyboardEvent => Result {} Str,
            handle_mouse_button_event! : MouseButtonEvent => Result {} Str,
            handle_mouse_drag_event! : MouseDragEvent => Result {} Str,
            handle_mouse_scroll_event! : MouseScrollEvent => Result {} Str,
        }
    }
    exposes [
        Command,
        Comp,
        Containers,
        Control,
        Entity,
        Input,
        Mesh,
        Model,
        Physics,
        Rendering,
        Scene,
        Setup,
        Skybox,
        Stdout,
        Texture,
        Voxel,
    ]
    packages {
        core: "../../../../roc_packages/core/main.roc",
    }
    imports []
    provides [
        setup_scene_extern!,
        handle_keyboard_event_extern!,
        handle_mouse_button_event_extern!,
        handle_mouse_drag_event_extern!,
        handle_mouse_scroll_event_extern!,
        command_roundtrip_extern!,
    ]

import Command.EngineCommand as EngineCommand
import Input.KeyboardEvent as KeyboardEvent exposing [KeyboardEvent]
import Input.MouseButtonEvent as MouseButtonEvent exposing [MouseButtonEvent]
import Input.MouseDragEvent as MouseDragEvent exposing [MouseDragEvent]
import Input.MouseScrollEvent as MouseScrollEvent exposing [MouseScrollEvent]

setup_scene_extern! : I32 => Result {} Str
setup_scene_extern! = |_|
    callbacks.setup_scene!({})

handle_keyboard_event_extern! : List U8 => Result {} Str
handle_keyboard_event_extern! = |bytes|
    event = KeyboardEvent.from_bytes(bytes) |> map_err_to_str?
    callbacks.handle_keyboard_event!(event)

handle_mouse_button_event_extern! : List U8 => Result {} Str
handle_mouse_button_event_extern! = |bytes|
    event = MouseButtonEvent.from_bytes(bytes) |> map_err_to_str?
    callbacks.handle_mouse_button_event!(event)

handle_mouse_drag_event_extern! : List U8 => Result {} Str
handle_mouse_drag_event_extern! = |bytes|
    event = MouseDragEvent.from_bytes(bytes) |> map_err_to_str?
    callbacks.handle_mouse_drag_event!(event)

handle_mouse_scroll_event_extern! : List U8 => Result {} Str
handle_mouse_scroll_event_extern! = |bytes|
    event = MouseScrollEvent.from_bytes(bytes) |> map_err_to_str?
    callbacks.handle_mouse_scroll_event!(event)

command_roundtrip_extern! : List U8 => Result (List U8) Str
command_roundtrip_extern! = |bytes|
    command = EngineCommand.from_bytes(bytes) |> map_err_to_str?
    Ok(EngineCommand.write_bytes([], command))

map_err_to_str = |result|
    result |> Result.map_err(|err| Inspect.to_str(err))
