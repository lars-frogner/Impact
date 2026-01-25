module [handle_event!]

import core.Vector3
import core.UnitVector3
import core.UnitQuaternion

import pf.Command
import pf.Game.InputContext exposing [InputContext]
import pf.Input.KeyboardEvent exposing [KeyboardEvent]

import pf.Comp.ReferenceFrame
import pf.Comp.Motion

import Entities.Player as Player
import Entities.Tools as Tools
import Entities.FreeCamera as FreeCamera
import Entities.OverviewCamera as OverviewCamera

handle_event! : InputContext, KeyboardEvent => Result {} Str
handle_event! = |ctx, event|
    when ctx.player_mode is
        Dynamic -> handle_event_dynamic_mode!(event)
        FreeCamera -> handle_event_free_camera_mode!(event)
        OverviewCamera -> handle_event_overview_camera_mode!(event)

handle_event_dynamic_mode! : KeyboardEvent => Result {} Str
handle_event_dynamic_mode! = |{ key, state }|
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
                    Space -> launch_projectile_dynamic_mode!(state)?
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
                    KeyM -> switch_to_dynamic_mode(state)
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
    force_magnitude = Tools.thruster.acceleration * Player.player.mass
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

launch_projectile_dynamic_mode! = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return Ok([])

    player_frame = Comp.ReferenceFrame.get_for_entity!(Player.entity_ids.player)?
    player_head_frame = Comp.ReferenceFrame.get_for_entity!(Player.entity_ids.player_head)?
    player_motion = Comp.Motion.get_for_entity!(Player.entity_ids.player)?

    position = Vector3.add(player_frame.position, UnitQuaternion.rotate_vector(player_frame.orientation, player_head_frame.position))
    orientation = UnitQuaternion.mul(player_frame.orientation, player_head_frame.orientation)

    reaction_impulse = Tools.spawn_projectile!(
        position,
        player_motion.linear_velocity,
        UnitQuaternion.rotate_vector(orientation, UnitVector3.neg_unit_z),
    )?

    apply_impulse = ApplyImpulse {
        entity_id: Player.entity_ids.player,
        impulse: reaction_impulse,
        relative_position: position,
    }

    Ok([Engine(Physics(apply_impulse))])

launch_projectile_free_camera_mode! = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return Ok([])

    frame = Comp.ReferenceFrame.get_for_entity!(FreeCamera.entity_ids.camera)?
    motion = Comp.Motion.get_for_entity!(FreeCamera.entity_ids.camera)?

    _ = Tools.spawn_projectile!(
        frame.position,
        motion.linear_velocity,
        UnitQuaternion.rotate_vector(frame.orientation, UnitVector3.neg_unit_z),
    )?

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

switch_to_dynamic_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetPlayerMode(Dynamic)),
        Engine(Scene(SetActiveCamera { entity_id: Player.entity_ids.player_head })),
    ]

switch_to_free_camera_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetPlayerMode(FreeCamera)),
        Engine(Scene(SetActiveCamera { entity_id: FreeCamera.entity_ids.camera })),
    ]

switch_to_overview_camera_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetPlayerMode(OverviewCamera)),
        UI(SetInteractivity(Enabled)),
        Engine(Scene(SetActiveCamera { entity_id: OverviewCamera.entity_ids.camera })),
    ]
