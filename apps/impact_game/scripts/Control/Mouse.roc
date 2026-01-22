module [
    handle_button_event!,
    handle_drag_event!,
    handle_scroll_event!,
]

import pf.Command
import pf.Entity
import pf.Game.InputContext exposing [InputContext]
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Input.MouseDragEvent exposing [MouseDragEvent]
import pf.Input.MouseScrollEvent exposing [MouseScrollEvent]
import pf.Input.MouseButtonState exposing [MouseButtonState]
import pf.Input.MouseButtonSet as Buttons

import Control.Overview
import Entities.Player as Player
import Entities.OverviewCamera as OverviewCamera

handle_button_event! : InputContext, MouseButtonEvent => Result {} Str
handle_button_event! = |ctx, event|
    when ctx.player_mode is
        Active -> handle_button_event_active_mode!(event)
        Overview -> handle_button_event_overview_mode!(event)

handle_drag_event! : InputContext, MouseDragEvent => Result {} Str
handle_drag_event! = |ctx, event|
    when ctx.player_mode is
        Active -> handle_drag_event_active_mode!(event)
        Overview -> handle_drag_event_overview_mode!(event)

handle_scroll_event! : InputContext, MouseScrollEvent => Result {} Str
handle_scroll_event! = |ctx, event|
    when ctx.player_mode is
        Active -> handle_scroll_event_active_mode!(event)
        Overview -> handle_scroll_event_overview_mode!(event)

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

handle_drag_event_active_mode! : MouseDragEvent => Result {} Str
handle_drag_event_active_mode! = |_event|
    Ok({})

handle_scroll_event_active_mode! : MouseScrollEvent => Result {} Str
handle_scroll_event_active_mode! = |_event|
    Ok({})

handle_button_event_overview_mode! : MouseButtonEvent => Result {} Str
handle_button_event_overview_mode! = |_event|
    Ok({})

handle_drag_event_overview_mode! : MouseDragEvent => Result {} Str
handle_drag_event_overview_mode! = |event|
    if Buttons.contains(event.pressed, Buttons.left) then
        Control.Overview.rotate_camera!(
            OverviewCamera.entity_ids.camera,
            OverviewCamera.camera.focus_position,
            Num.to_f32(event.ang_delta_x),
            Num.to_f32(event.ang_delta_y),
            Num.to_f32(event.cursor.ang_x),
            Num.to_f32(event.cursor.ang_y),
        )
    else if Buttons.contains(event.pressed, Buttons.right) then
        Control.Overview.pan_camera!(
            OverviewCamera.entity_ids.camera,
            OverviewCamera.camera.focus_position,
            Num.to_f32(event.ang_delta_x),
            Num.to_f32(event.ang_delta_y),
        )
    else
        Ok({})

handle_scroll_event_overview_mode! : MouseScrollEvent => Result {} Str
handle_scroll_event_overview_mode! = |event|
    Control.Overview.zoom_camera!(
        OverviewCamera.entity_ids.camera,
        OverviewCamera.camera.focus_position,
        Num.to_f32(event.delta_y),
    )

toggle_scene_entity_active_state! : Entity.Id, MouseButtonState => Result {} Str
toggle_scene_entity_active_state! = |entity_id, button_state|
    state =
        when button_state is
            Pressed -> Enabled
            Released -> Disabled
    Command.execute!(Engine(Scene(SetSceneEntityActiveState({ entity_id, state }))))
