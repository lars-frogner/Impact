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
                    KeyO -> on_released(state, Rendering(Postprocessing(SetAmbientOcclusion(Opposite))))
                    KeyY -> on_released(state, Rendering(Postprocessing(SetTemporalAntiAliasing(Opposite))))
                    KeyU -> on_released(state, Rendering(Postprocessing(SetBloom(Opposite))))
                    KeyT -> on_released(state, Rendering(Postprocessing(SetToneMappingMethod(Next))))
                    KeyX -> on_released(state, Rendering(Postprocessing(SetExposure(DifferentByStops(0.1)))))
                    KeyZ -> on_released(state, Rendering(Postprocessing(SetExposure(DifferentByStops(-0.1)))))
                    KeyV -> on_released(state, Rendering(Postprocessing(SetRenderAttachmentVisualization(Opposite))))
                    KeyB -> on_released(state, Rendering(Postprocessing(SetVisualizedRenderAttachmentQuantity(Next))))
                    KeyC -> on_released(state, Rendering(Postprocessing(SetVisualizedRenderAttachmentQuantity(Previous))))
                    KeyI -> on_released(state, Rendering(SetShadowMapping(Opposite)))
                    KeyF -> on_released(state, Rendering(SetWireframeMode(Opposite)))
                    KeyP -> on_released(state, Physics(SetSimulation(Opposite)))
                    KeyM -> on_released(state, Physics(SetSimulationSubstepCount(HigherBy(1))))
                    KeyN -> on_released(state, Physics(SetSimulationSubstepCount(LowerBy(1))))
                    _ -> None

            Symbol(symbol_key) ->
                when symbol_key is
                    Period -> on_released(state, Physics(SetSimulationSpeed(Higher)))
                    Comma -> on_released(state, Physics(SetSimulationSpeed(Lower)))
                    _ -> None

            # Whitespace(whitspace_key) ->
            #    when whitspace_key is
            #        Tab -> on_released(state, UI(Set(Opposite)))
            #        _ -> None
            Function(function_key) ->
                when function_key is
                    F12 -> on_released(state, Capture(SaveScreenshot))
                    F10 -> on_released(state, Capture(SaveShadowMaps(OmnidirectionalLight)))
                    F9 -> on_released(state, Capture(SaveShadowMaps(UnidirectionalLight)))
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

    Some(Control(SetMotion { direction, state }))

on_released = |state, command|
    when state is
        Released -> Some(command)
        Pressed -> None
