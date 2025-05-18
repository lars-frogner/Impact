module [toggle_scene_entity_active_state!]

import pf.Command
import pf.Entity
import pf.Input.MouseButtonState exposing [MouseButtonState]

toggle_scene_entity_active_state! : Entity.Id, MouseButtonState => Result {} Str
toggle_scene_entity_active_state! = |entity_id, button_state|
    state =
        when button_state is
            Pressed -> Enabled
            Released -> Disabled
    Command.execute!(Scene(SetSceneEntityActiveState({ entity_id, state })))
