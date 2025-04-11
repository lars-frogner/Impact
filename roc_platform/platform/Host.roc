hosted [
    stdout_line!,
    impact_run!,
    f32_to_bits!,
    f64_to_bits!,
    f32_from_bits!,
    f64_from_bits!,
]

import InternalIOErr

# Stdout
stdout_line! : Str => Result {} InternalIOErr.IOErrFromHost

# Impact
impact_run! : {} => Result {} Str

# Core
f32_to_bits! : F32 => U32
f64_to_bits! : F64 => U64
f32_from_bits! : U32 => F32
f64_from_bits! : U64 => F64
