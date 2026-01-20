platform "impact"
    requires {} {
        callbacks : {
            setup_scene! : SetupContext => Result {} Str,
            handle_keyboard_event! : InputContext, KeyboardEvent => Result {} Str,
            handle_mouse_button_event! : InputContext, MouseButtonEvent => Result {} Str,
            handle_mouse_drag_event! : InputContext, MouseDragEvent => Result {} Str,
            handle_mouse_scroll_event! : InputContext, MouseScrollEvent => Result {} Str,
        }
    }
    exposes [
        Command,
        Comp,
        Containers,
        Control,
        Entity,
        Game,
        Input,
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
        handle_mouse_button_event_extern!,
        handle_mouse_drag_event_extern!,
        handle_mouse_scroll_event_extern!,
    ]

import Command.EngineCommand as EngineCommand
import Game.SetupContext as SetupContext exposing [SetupContext]
import Game.InputContext as InputContext exposing [InputContext]
import Input.KeyboardEvent as KeyboardEvent exposing [KeyboardEvent]
import Input.MouseButtonEvent as MouseButtonEvent exposing [MouseButtonEvent]
import Input.MouseDragEvent as MouseDragEvent exposing [MouseDragEvent]
import Input.MouseScrollEvent as MouseScrollEvent exposing [MouseScrollEvent]

setup_scene_extern! : List U8 => Result {} Str
setup_scene_extern! = |ctx_bytes|
    ctx = SetupContext.from_bytes(ctx_bytes) |> map_err_to_str?
    callbacks.setup_scene!(ctx)

handle_keyboard_event_extern! : List U8, List U8 => Result {} Str
handle_keyboard_event_extern! = |ctx_bytes, event_bytes|
    ctx = InputContext.from_bytes(ctx_bytes) |> map_err_to_str?
    event = KeyboardEvent.from_bytes(event_bytes) |> map_err_to_str?
    callbacks.handle_keyboard_event!(ctx, event)

handle_mouse_button_event_extern! : List U8, List U8 => Result {} Str
handle_mouse_button_event_extern! = |ctx_bytes, event_bytes|
    ctx = InputContext.from_bytes(ctx_bytes) |> map_err_to_str?
    event = MouseButtonEvent.from_bytes(event_bytes) |> map_err_to_str?
    callbacks.handle_mouse_button_event!(ctx, event)

handle_mouse_drag_event_extern! : List U8, List U8 => Result {} Str
handle_mouse_drag_event_extern! = |ctx_bytes, event_bytes|
    ctx = InputContext.from_bytes(ctx_bytes) |> map_err_to_str?
    event = MouseDragEvent.from_bytes(event_bytes) |> map_err_to_str?
    callbacks.handle_mouse_drag_event!(ctx, event)

handle_mouse_scroll_event_extern! : List U8, List U8 => Result {} Str
handle_mouse_scroll_event_extern! = |ctx_bytes, event_bytes|
    ctx = InputContext.from_bytes(ctx_bytes) |> map_err_to_str?
    event = MouseScrollEvent.from_bytes(event_bytes) |> map_err_to_str?
    callbacks.handle_mouse_scroll_event!(ctx, event)

map_err_to_str = |result|
    result |> Result.map_err(|err| Inspect.to_str(err))
