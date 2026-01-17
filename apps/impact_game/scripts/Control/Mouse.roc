module [
    handle_button_event!,
    handle_drag_event!,
    handle_scroll_event!,
]

import pf.Command
import pf.Entity
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Input.MouseDragEvent exposing [MouseDragEvent]
import pf.Input.MouseScrollEvent exposing [MouseScrollEvent]
import pf.Input.MouseButtonState exposing [MouseButtonState]

import Entities.Player as Player

handle_button_event! : Player.PlayerMode, MouseButtonEvent => Result {} Str
handle_button_event! = |player_mode, event|
    when player_mode is
        Active -> handle_button_event_active_mode!(event)
        Overview -> handle_button_event_overview_mode!(event)

handle_button_event_active_mode! : MouseButtonEvent => Result {} Str
handle_button_event_active_mode! = |{ button, state }|
    when button is
        Left ->
            toggle_scene_entity_active_state!(
                Player.entity_ids.laser,
                state,
            )

        Right ->
            toggle_scene_entity_active_state!(
                Player.entity_ids.absorbing_sphere,
                state,
            )

        _ -> Ok({})

handle_button_event_overview_mode! : MouseButtonEvent => Result {} Str
handle_button_event_overview_mode! = |_event|
    Ok({})

handle_drag_event! : MouseDragEvent => Result {} Str
handle_drag_event! = |_event|
    Ok({})

handle_scroll_event! : MouseScrollEvent => Result {} Str
handle_scroll_event! = |_event|
    Ok({})

toggle_scene_entity_active_state! : Entity.Id, MouseButtonState => Result {} Str
toggle_scene_entity_active_state! = |entity_id, button_state|
    state =
        when button_state is
            Pressed -> Enabled
            Released -> Disabled
    Command.execute!(Engine(Scene(SetSceneEntityActiveState({ entity_id, state }))))
