module [
    handle_button_event!,
    handle_drag_event!,
    handle_scroll_event!,
]

import pf.Command
import pf.Game.InputContext exposing [InputContext]
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import pf.Input.MouseDragEvent exposing [MouseDragEvent]
import pf.Input.MouseScrollEvent exposing [MouseScrollEvent]
import pf.Input.MouseButtonSet as Buttons

import Control.Overview
import Entities.Player as Player
import Entities.FreeCamera as FreeCamera
import Entities.OverviewCamera as OverviewCamera

handle_button_event! : InputContext, MouseButtonEvent => Result {} Str
handle_button_event! = |ctx, event|
    when ctx.player_mode is
        Dynamic -> handle_button_event_dynamic_mode!(event)
        FreeCamera -> handle_button_event_free_camera_mode!(event)
        OverviewCamera -> Ok({})

handle_drag_event! : InputContext, MouseDragEvent => Result {} Str
handle_drag_event! = |ctx, event|
    when ctx.player_mode is
        Dynamic | FreeCamera -> Ok({})
        OverviewCamera -> handle_drag_event_overview_camera_mode!(event)

handle_scroll_event! : InputContext, MouseScrollEvent => Result {} Str
handle_scroll_event! = |ctx, event|
    when ctx.player_mode is
        Dynamic | FreeCamera -> Ok({})
        OverviewCamera -> handle_scroll_event_overview_camera_mode!(event)

handle_button_event_dynamic_mode! = |{ button, state }|
    when button is
        Left ->
            toggle_scene_entity_active_state!(
                Player.entity_ids.tools.laser,
                state,
            )

        Right ->
            toggle_scene_entity_active_state!(
                Player.entity_ids.tools.absorbing_sphere,
                state,
            )

        _ -> Ok({})

handle_button_event_free_camera_mode! = |{ button, state }|
    when button is
        Left ->
            toggle_scene_entity_active_state!(
                FreeCamera.entity_ids.tools.laser,
                state,
            )

        Right ->
            toggle_scene_entity_active_state!(
                FreeCamera.entity_ids.tools.absorbing_sphere,
                state,
            )

        _ -> Ok({})

handle_drag_event_overview_camera_mode! = |event|
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

handle_scroll_event_overview_camera_mode! = |event|
    Control.Overview.zoom_camera!(
        OverviewCamera.entity_ids.camera,
        OverviewCamera.camera.focus_position,
        Num.to_f32(event.delta_y),
    )

toggle_scene_entity_active_state! = |entity_id, button_state|
    state =
        when button_state is
            Pressed -> Enabled
            Released -> Disabled
    Command.execute!(Engine(Scene(SetSceneEntityActiveState({ entity_id, state }))))
