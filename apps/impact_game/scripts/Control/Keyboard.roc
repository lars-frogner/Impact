module [handle_event!]

import core.UnitVector3

import pf.Command
import pf.Input.KeyboardEvent exposing [KeyboardEvent]

import pf.Comp.LocalForceGeneratorID
import pf.Comp.AlignmentTorqueGeneratorID

import Entities.Player as Player
import Entities.Tools as Tools

handle_event! : Player.PlayerMode, KeyboardEvent => Result {} Str
handle_event! = |player_mode, event|
    when player_mode is
        Active -> handle_event_active_mode!(event)
        Overview -> handle_event_overview_mode!(event)

handle_event_active_mode! : KeyboardEvent => Result {} Str
handle_event_active_mode! = |{ key, state }|
    command =
        when key is
            Control(control_key) ->
                when control_key is
                    Escape -> toggle_interactivity(state)
                    _ -> None

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
                    _ -> None

            _ -> None

    when command is
        Some(comm) -> Command.execute!(comm)
        None -> Ok({})

handle_event_overview_mode! : KeyboardEvent => Result {} Str
handle_event_overview_mode! = |_event|
    Ok({})

toggle_interactivity = |key_state|
    when key_state is
        Released -> {}
        _ ->
            return None

    Some(UI(SetInteractivity(Opposite)))

add_thruster_force! = |key_state, direction|
    force_magnitude = Tools.thruster.acceleration * Player.player.mass
    force =
        when key_state is
            Pressed -> force_magnitude
            Released -> -force_magnitude
            Held ->
                return Ok(None)

    force_vector =
        when direction is
            Forwards -> (0, 0, force)
            Backwards -> (0, 0, -force)
            Left -> (-force, 0, 0)
            Right -> (force, 0, 0)
            Down -> (0, -force, 0)
            Up -> (0, force, 0)

    generator_id = Comp.LocalForceGeneratorID.get_for_entity!(Player.entity_ids.player)?
    Ok(Some(Engine(Physics(UpdateLocalForce { generator_id, mode: Add, force: force_vector }))))

set_alignment_direction! = |key_state, direction|
    when key_state is
        Pressed -> {}
        _ ->
            return Ok(None)

    generator_id = Comp.AlignmentTorqueGeneratorID.get_for_entity!(Player.entity_ids.player)?
    Ok(Some(Engine(Physics(SetAlignmentTorqueDirection { generator_id, direction }))))
