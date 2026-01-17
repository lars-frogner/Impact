app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import Generation.SolarSystem
import Scenes.SolarSystem
import Control.Keyboard
import Control.Mouse

callbacks = {
    setup_scene!,
    handle_keyboard_event!,
    handle_mouse_button_event!,
    handle_mouse_drag_event!,
    handle_mouse_scroll_event!,
}

player_mode = Overview

setup_scene! : {} => Result {} Str
setup_scene! = |_|
    system = Generation.SolarSystem.generate(
        {
            number_of_bodies: 100,
            body_size_distr: { exponent: -2, min_value: 10.0, max_value: 3e2 },
            body_distance_distr: { exponent: -1, min_value: 1e3, max_value: 5e3 },
            star_radius: 3e2,
            star_mass_density: 5e4,
            max_orbital_period: 2 * 60.0,
            min_body_illuminance: 1e4,
        },
        0,
    )
    Scenes.SolarSystem.setup!(system, player_mode)

handle_keyboard_event! = |event|
    Control.Keyboard.handle_event!(player_mode, event)

handle_mouse_button_event! = |event|
    Control.Mouse.handle_button_event!(player_mode, event)

handle_mouse_drag_event! = |event|
    Control.Mouse.handle_drag_event!(player_mode, event)

handle_mouse_scroll_event! = |event|
    Control.Mouse.handle_scroll_event!(player_mode, event)
