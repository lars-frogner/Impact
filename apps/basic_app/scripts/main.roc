app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Stdout
import InputHandling.Keyboard as KeyboardInput
import Scenes.Asteroid

callbacks = {
    setup_scene!: Scenes.Asteroid.setup!,
    handle_keyboard_event!: KeyboardInput.handle_event!,
    handle_mouse_button_event!,
}

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |{ button, state }|
    _ = Stdout.line!("Handling mouse button ${Inspect.to_str(button)} ${Inspect.to_str(state)}")
    Scenes.Asteroid.handle_mouse_button_event!({ button, state })
