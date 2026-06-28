app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Input.KeyboardEvent exposing [KeyboardEvent]
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Input.MouseDragEvent exposing [MouseDragEvent]
import pf.Input.MouseScrollEvent exposing [MouseScrollEvent]
# import InputHandling.Keyboard as KeyboardInput
import PhysicsExperiments.Fracturing

callbacks = {
    setup_scene!: PhysicsExperiments.Fracturing.setup!,
    handle_keyboard_event!,
    handle_mouse_button_event!,
    handle_mouse_drag_event!,
    handle_mouse_scroll_event!,
}

handle_keyboard_event! : KeyboardEvent => Result {} Str
handle_keyboard_event! = |event|
    PhysicsExperiments.Fracturing.handle_keyboard_event!(event)

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |event|
    PhysicsExperiments.Fracturing.handle_mouse_button_event!(event)

handle_mouse_drag_event! : MouseDragEvent => Result {} Str
handle_mouse_drag_event! = |_event|
    Ok({})

handle_mouse_scroll_event! : MouseScrollEvent => Result {} Str
handle_mouse_scroll_event! = |_event|
    Ok({})
