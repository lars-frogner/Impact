hosted [
    stdout_line!,
    execute_engine_command!,
    create_entity!,
    create_entities!,
]

import InternalIOErr

# Stdout
stdout_line! : Str => Result {} InternalIOErr.IOErrFromHost

# Impact
execute_engine_command! : List U8 => Result {} Str
create_entity! : List U8 => Result U64 Str
create_entities! : List U8 => Result (List U64) Str
