hosted [
    lookup_game_target!,
    execute_game_command!,
    execute_ui_command!,
    execute_engine_command!,
    stage_entity_for_creation_with_id!,
    stage_entity_for_creation!,
    stage_entities_for_creation!,
    stage_entity_for_update!,
    stage_entity_for_removal!,
    create_entity_with_id!,
    create_entity!,
    create_entities!,
    update_entity!,
    remove_entity!,
    read_entity_components!,
    stdout_line!,
]

import InternalIOErr

# Game
lookup_game_target! : List U8 => Result (List U8) Str
execute_game_command! : List U8 => Result {} Str

# UI
execute_ui_command! : List U8 => Result {} Str

# Engine
execute_engine_command! : List U8 => Result {} Str
stage_entity_for_creation_with_id! : U64, List U8 => Result {} Str
stage_entity_for_creation! : List U8 => Result {} Str
stage_entities_for_creation! : List U8 => Result {} Str
stage_entity_for_update! : U64, List U8 => Result {} Str
stage_entity_for_removal! : U64 => Result {} Str
create_entity_with_id! : U64, List U8 => Result {} Str
create_entity! : List U8 => Result U64 Str
create_entities! : List U8 => Result (List U64) Str
update_entity! : U64, List U8 => Result {} Str
remove_entity! : U64 => Result {} Str
read_entity_components! : U64, List U64 => Result (List U8) Str

# Stdout
stdout_line! : Str => Result {} InternalIOErr.IOErrFromHost
