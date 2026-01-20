module [handle_event!]

import core.UnitVector3

import pf.Command
import pf.Game.InputContext exposing [InputContext]
import pf.Input.KeyboardEvent exposing [KeyboardEvent]

import pf.Comp.LocalForceGeneratorID
import pf.Comp.AlignmentTorqueGeneratorID

import Entities.Player as Player
import Entities.Tools as Tools
import Entities.OverviewCamera as OverviewCamera

handle_event! : InputContext, KeyboardEvent => Result {} Str
handle_event! = |ctx, event|
    when ctx.player_mode is
        Active -> handle_event_active_mode!(event)
        Overview -> handle_event_overview_mode!(event)

handle_event_active_mode! : KeyboardEvent => Result {} Str
handle_event_active_mode! = |{ key, state }|
    commands =
        when key is
            Control(control_key) ->
                when control_key is
                    Escape -> set_ui_interactivity(state, Opposite)
                    _ -> []

            Letter(letter_key) ->
                when letter_key is
                    KeyW -> add_thruster_force!(state, Forwards)?
                    KeyS -> add_thruster_force!(state, Backwards)?
                    KeyD -> add_thruster_force!(state, Left)?
                    KeyA -> add_thruster_force!(state, Right)?
                    KeyQ -> add_thruster_force!(state, Down)?
                    KeyE -> add_thruster_force!(state, Up)?
                    KeyY -> set_alignment_direction!(state, Fixed(UnitVector3.neg_unit_y))?
                    KeyG -> set_alignment_direction!(state, GravityForce)?
                    KeyO -> switch_to_overview_mode(state)
                    _ -> []

            _ -> []

    commands |> List.for_each_try!(Command.execute!)

handle_event_overview_mode! : KeyboardEvent => Result {} Str
handle_event_overview_mode! = |{ key, state }|
    commands =
        when key is
            Letter(letter_key) ->
                when letter_key is
                    KeyO -> switch_to_active_mode(state)
                    _ -> []

            _ -> []

    commands |> List.for_each_try!(Command.execute!)

set_ui_interactivity = |key_state, to|
    when key_state is
        Released -> {}
        _ ->
            return []

    [UI(SetInteractivity(to))]

add_thruster_force! = |key_state, direction|
    force_magnitude = Tools.thruster.acceleration * Player.player.mass
    force =
        when key_state is
            Pressed -> force_magnitude
            Released -> -force_magnitude
            Held ->
                return Ok([])

    force_vector =
        when direction is
            Forwards -> (0, 0, force)
            Backwards -> (0, 0, -force)
            Left -> (-force, 0, 0)
            Right -> (force, 0, 0)
            Down -> (0, -force, 0)
            Up -> (0, force, 0)

    generator_id = Comp.LocalForceGeneratorID.get_for_entity!(Player.entity_ids.player)?
    Ok([Engine(Physics(UpdateLocalForce { generator_id, mode: Add, force: force_vector }))])

set_alignment_direction! = |key_state, direction|
    when key_state is
        Released -> {}
        _ ->
            return Ok([])

    generator_id = Comp.AlignmentTorqueGeneratorID.get_for_entity!(Player.entity_ids.player)?
    Ok([Engine(Physics(SetAlignmentTorqueDirection { generator_id, direction }))])

switch_to_overview_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetPlayerMode(Overview)),
        UI(SetInteractivity(Enabled)),
        Engine(Scene(SetActiveCamera { entity_id: OverviewCamera.entity_ids.camera })),
    ]

switch_to_active_mode = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return []

    [
        Game(SetPlayerMode(Active)),
        Engine(Scene(SetActiveCamera { entity_id: Player.entity_ids.player_head })),
    ]
