app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Input.MouseDragEvent exposing [MouseDragEvent]
import pf.Input.MouseScrollEvent exposing [MouseScrollEvent]
import InputHandling.Keyboard as KeyboardInput
import Scenes.SunOnly

callbacks = {
    setup_scene!: Scenes.SunOnly.setup!,
    handle_keyboard_event!: KeyboardInput.handle_event!,
    handle_mouse_button_event!,
    handle_mouse_drag_event!,
    handle_mouse_scroll_event!,
}

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |event|
    Scenes.SunOnly.handle_mouse_button_event!(event)

handle_mouse_drag_event! : MouseDragEvent => Result {} Str
handle_mouse_drag_event! = |_event|
    Ok({})

handle_mouse_scroll_event! : MouseScrollEvent => Result {} Str
handle_mouse_scroll_event! = |_event|
    Ok({})
