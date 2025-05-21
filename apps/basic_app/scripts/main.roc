app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Stdout
import InputHandling.Keyboard as KeyboardInput
import InputHandling.MouseButton as MouseButtonInput
import Scenes.RenderingTest

callbacks = {
    setup_scene!: Scenes.RenderingTest.setup!,
    handle_keyboard_event!: KeyboardInput.handle_event!,
    handle_mouse_button_event!,
}
entities = Scenes.RenderingTest.entities

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |{ button, state }|
    _ = Stdout.line!("Handling mouse button ${Inspect.to_str(button)} ${Inspect.to_str(state)}")
    when button is
        Left -> MouseButtonInput.toggle_scene_entity_active_state!(entities.dragon, flip(state))
        _ -> Ok({})

flip = |state|
    when state is
        Pressed -> Released
        Released -> Pressed
