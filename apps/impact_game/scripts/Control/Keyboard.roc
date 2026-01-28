module [handle_event!]

import core.UnitVector3

import pf.Command
import pf.Game.InputContext exposing [InputContext]
import pf.Input.KeyboardEvent exposing [KeyboardEvent]

import Entities.Player as Player
import Entities.Tools as Tools
import Entities.FreeCamera as FreeCamera
import Entities.OverviewCamera as OverviewCamera

handle_event! : InputContext, KeyboardEvent => Result {} Str
handle_event! = |ctx, event|
    when ctx.interaction_mode is
        Player -> handle_event_player_mode!(event)
        FreeCamera -> handle_event_free_camera_mode!(event)
        OverviewCamera -> handle_event_overview_camera_mode!(event)

handle_event_player_mode! : KeyboardEvent => Result {} Str
handle_event_player_mode! = |{ key, state }|
    commands =
        when key is
            Control(control_key) ->
                when control_key is
                    Escape -> set_ui_interactivity(state, Opposite)
                    _ -> []

            Letter(letter_key) ->
                when letter_key is
                    KeyW -> add_thruster_force(state, Forwards)
                    KeyS -> add_thruster_force(state, Backwards)
                    KeyD -> add_thruster_force(state, Left)
                    KeyA -> add_thruster_force(state, Right)
                    KeyQ -> add_thruster_force(state, Down)
                    KeyE -> add_thruster_force(state, Up)
                    KeyY -> set_alignment_direction(state, Fixed(UnitVector3.neg_unit_y))
                    KeyG -> set_alignment_direction(state, GravityForce)
                    KeyM -> switch_to_free_camera_mode(state)
                    _ -> []

            Whitespace(whitespace_key) ->
                when whitespace_key is
                    Space -> launch_projectile_player_mode!(state)?
                    _ -> []

            _ -> []

    commands |> List.for_each_try!(Command.execute!)

handle_event_free_camera_mode! : KeyboardEvent => Result {} Str
handle_event_free_camera_mode! = |{ key, state }|
    commands =
        when key is
            Control(control_key) ->
                when control_key is
                    Escape -> set_ui_interactivity(state, Opposite)
                    _ -> []

            Letter(letter_key) ->
                when letter_key is
                    KeyW -> set_motion(state, Backwards)?
                    KeyS -> set_motion(state, Forwards)?
                    KeyD -> set_motion(state, Right)?
                    KeyA -> set_motion(state, Left)?
                    KeyQ -> set_motion(state, Down)?
                    KeyE -> set_motion(state, Up)?
                    KeyM -> switch_to_overview_camera_mode(state)
                    _ -> []

            Whitespace(whitespace_key) ->
                when whitespace_key is
                    Space -> launch_projectile_free_camera_mode!(state)?
                    _ -> []

            _ -> []

    commands |> List.for_each_try!(Command.execute!)

handle_event_overview_camera_mode! : KeyboardEvent => Result {} Str
handle_event_overview_camera_mode! = |{ key, state }|
    commands =
        when key is
            Letter(letter_key) ->
                when letter_key is
                    KeyM -> switch_to_player_mode(state)
                    _ -> []

            _ -> []

    commands |> List.for_each_try!(Command.execute!)

set_ui_interactivity = |key_state, to|
    when key_state is
        Released -> {}
        _ ->
            return []

    [UI(SetInteractivity(to))]

add_thruster_force = |key_state, direction|
    force_magnitude = Tools.thruster.force
    force =
        when key_state is
            Pressed -> force_magnitude
            Released -> -force_magnitude
            Held ->
                return []

    force_vector =
        when direction is
            Forwards -> (0, 0, force)
            Backwards -> (0, 0, -force)
            Left -> (-force, 0, 0)
            Right -> (force, 0, 0)
            Down -> (0, -force, 0)
            Up -> (0, force, 0)

    [Engine(Physics(UpdateLocalForce { entity_id: Player.entity_ids.player, mode: Add, force: force_vector }))]

launch_projectile_player_mode! = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return Ok([])

    Player.launch_projectile!({})

launch_projectile_free_camera_mode! = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return Ok([])

    FreeCamera.launch_projectile!({})?

    Ok([])

set_motion = |key_state, direction|
    state =
        when key_state is
            Pressed -> Moving
            Released -> Still
            Held ->
                return Ok([])

    Ok([Engine(Control(SetMotion { direction, state }))])

set_alignment_direction = |key_state, direction|
    when key_state is
        Released -> {}
        _ ->
            return []

    [Engine(Physics(SetAlignmentTorqueDirection { entity_id: Player.entity_ids.player, direction }))]

switch_to_player_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetInteractionMode(Player)),
        Engine(Scene(SetActiveCamera { entity_id: Player.entity_ids.player_head })),
    ]

switch_to_free_camera_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetInteractionMode(FreeCamera)),
        Engine(Scene(SetActiveCamera { entity_id: FreeCamera.entity_ids.camera })),
    ]

switch_to_overview_camera_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetInteractionMode(OverviewCamera)),
        UI(SetInteractivity(Enabled)),
        Engine(Scene(SetActiveCamera { entity_id: OverviewCamera.entity_ids.camera })),
    ]
