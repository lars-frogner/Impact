# Hash: f2c7b849f8af8cdc35863f15bc3e1cf78bcc1ecdb198bee6fedb0085fb3000fc
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::physics::material::ContactResponseParameters
# Type category: POD
# Commit: 31f3514 (dirty)
module [
    ContactResponseParameters,
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
    restitution_coef : F64,
    ## The strength of friction at the contact when the touching surfaces are
    ## not sliding across each other.
    static_friction_coef : F64,
    ## The strength of friction at the contact when the touching surfaces are
    ## sliding across each other.
    dynamic_friction_coef : F64,
}

## Serializes a value of [ContactResponseParameters] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ContactResponseParameters -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Builtin.write_bytes_f64(value.restitution_coef)
    |> Builtin.write_bytes_f64(value.static_friction_coef)
    |> Builtin.write_bytes_f64(value.dynamic_friction_coef)

## Deserializes a value of [ContactResponseParameters] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ContactResponseParameters _
from_bytes = |bytes|
    Ok(
        {
            restitution_coef: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            static_friction_coef: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            dynamic_friction_coef: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
