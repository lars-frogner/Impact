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
                    KeyP -> on_released(state, Engine(GameLoop(SetGameLoop(Opposite))))
                    _ -> None

            Symbol(symbol_key) ->
                when symbol_key is
                    Period -> on_released(state, Engine(GameLoop(PauseAfterSingleIteration)))
                    _ -> None

            Function(function_key) ->
                when function_key is
                    F12 -> on_released(state, Engine(Capture(SaveScreenshot)))
                    F10 -> on_released(state, Engine(Capture(SaveShadowMaps(OmnidirectionalLight))))
                    F9 -> on_released(state, Engine(Capture(SaveShadowMaps(UnidirectionalLight))))
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
