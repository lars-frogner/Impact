hosted [
    stdout_line!,
    create_entity!,
    create_entities!,
    set_skybox!,
    enable_scene_entity!,
    disable_scene_entity!,
]

import InternalIOErr

# Stdout
stdout_line! : Str => Result {} InternalIOErr.IOErrFromHost

# Impact
create_entity! : List U8 => Result U64 Str
create_entities! : List U8 => Result (List U64) Str
set_skybox! : Str, F32 => Result {} Str
enable_scene_entity! : U64 => Result {} Str
disable_scene_entity! : U64 => Result {} Str
