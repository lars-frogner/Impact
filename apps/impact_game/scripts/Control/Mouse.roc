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
import Entities.Tools as Tools

handle_button_event! : InputContext, MouseButtonEvent => Result {} Str
handle_button_event! = |ctx, event|
    when ctx.interaction_mode is
        Player -> handle_button_event_player_mode!(event)
        FreeCamera -> handle_button_event_free_camera_mode!(event)
        OverviewCamera -> Ok({})

handle_drag_event! : InputContext, MouseDragEvent => Result {} Str
handle_drag_event! = |ctx, event|
    when ctx.interaction_mode is
        Player | FreeCamera -> Ok({})
        OverviewCamera -> handle_drag_event_overview_camera_mode!(event)

handle_scroll_event! : InputContext, MouseScrollEvent => Result {} Str
handle_scroll_event! = |ctx, event|
    when ctx.interaction_mode is
        Player | FreeCamera -> handle_scroll_event_player_and_free_camera_mode!(event)
        OverviewCamera -> handle_scroll_event_overview_camera_mode!(event)

handle_button_event_player_mode! = |{ button, state }|
    when button is
        Left ->
            toggle_scene_entity_active_states!(
                [
                    Player.entity_ids.tools.laser,
                    Player.entity_ids.tools.laser_model,
                ],
                state,
            )

        Right ->
            toggle_scene_entity_active_states!(
                [
                    Player.entity_ids.tools.absorber,
                    Player.entity_ids.tools.absorber_model,
                ],
                state,
            )

        _ -> Ok({})

handle_scroll_event_player_and_free_camera_mode! = |event|
    Tools.adjust_launch_speed!(Num.to_f32(event.delta_y))

handle_button_event_free_camera_mode! = |{ button, state }|
    when button is
        Left ->
            toggle_scene_entity_active_states!(
                [
                    FreeCamera.entity_ids.tools.laser,
                    FreeCamera.entity_ids.tools.laser_model,
                ],
                state,
            )

        Right ->
            toggle_scene_entity_active_states!(
                [
                    FreeCamera.entity_ids.tools.absorber,
                    FreeCamera.entity_ids.tools.absorber_model,
                ],
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

toggle_scene_entity_active_states! = |entity_ids, button_state|
    state =
        when button_state is
            Pressed -> Enabled
            Released -> Disabled

    commands =
        entity_ids
        |> List.map(|entity_id| Engine(Scene(SetSceneEntityActiveState({ entity_id, state }))))

    commands |> List.for_each_try!(Command.execute!)
