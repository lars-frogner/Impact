module [handle_event!]

import pf.Command
import pf.Input.KeyboardEvent exposing [KeyboardEvent]

handle_event! : KeyboardEvent => Result {} Str
handle_event! = |{ key, state }|
    command =
        when key is
            Letter(letter_key) ->
                when letter_key is
                    KeyW -> set_motion(state, Backwards) # Camera looks backwards
                    KeyS -> set_motion(state, Forwards)
                    KeyA -> set_motion(state, Left)
                    KeyD -> set_motion(state, Right)
                    KeyQ -> set_motion(state, Up)
                    KeyE -> set_motion(state, Down)
                    _ -> None

            Control(control_key) ->
                when control_key is
                    Escape -> on_released(state, UI(SetInteractivity(Opposite)))
                    _ -> None

            _ -> None

    when command is
        Some(comm) -> Command.execute!(comm)
        None -> Ok({})

set_motion = |key_state, direction|
    state =
        when key_state is
            Pressed -> Moving
            Released -> Still

    Some(Engine(Control(SetMotion { direction, state })))

on_released = |state, command|
    when state is
        Released -> Some(command)
        Pressed -> None
