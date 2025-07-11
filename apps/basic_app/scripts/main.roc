app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import InputHandling.Keyboard as KeyboardInput
import Scenes.RenderingTest

callbacks = {
    setup_scene!: Scenes.RenderingTest.setup!,
    handle_keyboard_event!: KeyboardInput.handle_event!,
    handle_mouse_button_event!,
}

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |_event|
    # Scenes.Asteroid.handle_mouse_button_event!(event)
    Ok({})
