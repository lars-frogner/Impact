# Hash: c6c33c3e5c566073
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact_physics::material::ContactResponseParameters
# Type category: POD
module [
    ContactResponseParameters,
    new,
    write_bytes,
    from_bytes,
]

import core.Builtin

## Parameters quantifying the physical response of a body in contact with
## another body.
ContactResponseParameters : {
    ## The elasticity of collisions with the body, typically between 0 (fully
    ## inelastic, the bodies stay together) and 1 (elastic, the bodies bounce
    ## maximally apart).
    restitution_coef : F32,
    ## The strength of friction at the contact when the touching surfaces are
    ## not sliding across each other.
    static_friction_coef : F32,
    ## The strength of friction at the contact when the touching surfaces are
    ## sliding across each other.
    dynamic_friction_coef : F32,
}

new : F32, F32, F32 -> ContactResponseParameters
new = |restitution_coef, static_friction_coef, dynamic_friction_coef|
    { restitution_coef, static_friction_coef, dynamic_friction_coef }

## Serializes a value of [ContactResponseParameters] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ContactResponseParameters -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32(value.restitution_coef)
    |> Builtin.write_bytes_f32(value.static_friction_coef)
    |> Builtin.write_bytes_f32(value.dynamic_friction_coef)

## Deserializes a value of [ContactResponseParameters] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ContactResponseParameters _
from_bytes = |bytes|
    Ok(
        {
            restitution_coef: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            static_friction_coef: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            dynamic_friction_coef: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 12 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
