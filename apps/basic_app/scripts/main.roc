app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Command
import pf.Entity
import pf.Input.KeyboardEvent exposing [KeyboardEvent]
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Skybox
import pf.Stdout
import core.Hashing
import TestScene

callbacks = {
    setup_scene!,
    handle_keyboard_event!,
    handle_mouse_button_event!,
}

setup_scene! : {} => Result {} Str
setup_scene! = |_|
    _ = Stdout.line!("setup_scene! called")

    _player = Entity.create!(TestScene.player({}))?
    _ground = Entity.create!(TestScene.ground({}))?
    _ambient_light = Entity.create!(TestScene.ambient_light({}))?
    _unidirectional_light = Entity.create!(TestScene.unidirectional_light({}))?

    Command.execute!(
        Scene(
            SetSkybox(
                Skybox.new(Hashing.hash_str_32("space_skybox"), 1e6),
            ),
        ),
    )?

    Ok({})

handle_keyboard_event! : KeyboardEvent => Result {} Str
handle_keyboard_event! = |event|
    _ = Stdout.line!("handle_keyboard_event! called with event ${Inspect.to_str(event)}")
    Ok({})

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |event|
    _ = Stdout.line!("handle_mouse_button_event! called with event ${Inspect.to_str(event)}")
    Ok({})
