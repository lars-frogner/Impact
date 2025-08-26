hosted [
    execute_engine_command!,
    stage_entity_for_creation_with_id!,
    stage_entity_for_creation!,
    stage_entities_for_creation!,
    stage_entity_for_removal!,
    create_entity_with_id!,
    create_entity!,
    create_entities!,
    remove_entity!,
    stdout_line!,
]

import InternalIOErr

# Engine
execute_engine_command! : List U8 => Result {} Str
stage_entity_for_creation_with_id! : U64, List U8 => Result {} Str
stage_entity_for_creation! : List U8 => Result {} Str
stage_entities_for_creation! : List U8 => Result {} Str
stage_entity_for_removal! : U64 => Result {} Str
create_entity_with_id! : U64, List U8 => Result {} Str
create_entity! : List U8 => Result U64 Str
create_entities! : List U8 => Result (List U64) Str
remove_entity! : U64 => Result {} Str

# Stdout
stdout_line! : Str => Result {} InternalIOErr.IOErrFromHost
