app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Command
import pf.Entity
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Skybox
import pf.Stdout
import core.Hashing
import InputHandling.Keyboard as KeyboardInput
import InputHandling.MouseButton as MouseButtonInput
import Scenes.Test as TestScene

callbacks = {
    setup_scene!,
    handle_keyboard_event!: KeyboardInput.handle_event!,
    handle_mouse_button_event!,
}

setup_scene! : {} => Result {} Str
setup_scene! = |_|
    _ = Stdout.line!("Setting up scene")

    Entity.create_with_id!(TestScene.player_id, TestScene.player({}))?
    Entity.create_with_id!(TestScene.ground_id, TestScene.ground({}))?
    _ = Entity.create!(TestScene.ambient_light({}))?
    Entity.create_with_id!(TestScene.unidirectional_light_id, TestScene.unidirectional_light({}))?

    Command.execute!(
        Scene(
            SetSkybox(
                Skybox.new(Hashing.hash_str_32("space_skybox"), 1e6),
            ),
        ),
    )?

    Ok({})

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |{ button, state }|
    _ = Stdout.line!("Handling mouse button ${Inspect.to_str(button)} ${Inspect.to_str(state)}")
    when button is
        Left -> MouseButtonInput.toggle_scene_entity_active_state!(TestScene.ground_id, flip(state))
        _ -> Ok({})

flip = |state|
    when state is
        Pressed -> Released
        Released -> Pressed
