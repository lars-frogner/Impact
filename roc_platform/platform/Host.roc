hosted [
    stdout_line!,
    impact_run!,
    create_entity!,
    create_entities!,
]

import InternalIOErr

# Stdout
stdout_line! : Str => Result {} InternalIOErr.IOErrFromHost

# Impact
impact_run! : {} => Result {} Str
create_entity! : List U8 => Result U64 Str
create_entities! : List U8 => Result (List U64) Str
