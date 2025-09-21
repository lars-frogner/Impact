app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import InputHandling.Keyboard as KeyboardInput
import InputHandling.Mouse as MouseInput
import Scene

callbacks = {
    setup_scene!: Scene.setup!,
    handle_keyboard_event!: KeyboardInput.handle_event!,
    handle_mouse_button_event!: MouseInput.handle_button_event!,
    handle_mouse_drag_event!: MouseInput.handle_drag_event!,
    handle_mouse_scroll_event!: MouseInput.handle_scroll_event!,
}
