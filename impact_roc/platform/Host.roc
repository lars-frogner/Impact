hosted [
    stdout_line!,
    impact_run!,
    f32_to_bits!,
    f64_to_bits!,
    f32_from_bits!,
    f64_from_bits!,
    vec3_f32_roundtrip!,
    vec4_f32_roundtrip!,
    vec3_f64_roundtrip!,
    vec4_f64_roundtrip!,
    test_struct_roundtrip!,
]

import InternalIOErr

# Stdout
stdout_line! : Str => Result {} InternalIOErr.IOErrFromHost

# Impact
impact_run! : {} => Result {} Str

# Rosetta
f32_to_bits! : F32 => U32
f64_to_bits! : F64 => U64
f32_from_bits! : U32 => F32
f64_from_bits! : U64 => F64

vec3_f32_roundtrip! : List U8 => Result (List U8) Str
vec4_f32_roundtrip! : List U8 => Result (List U8) Str
vec3_f64_roundtrip! : List U8 => Result (List U8) Str
vec4_f64_roundtrip! : List U8 => Result (List U8) Str
test_struct_roundtrip! : List U8 => Result (List U8) Str
