# Hash: b00cbb08d785052a53328ed6f0b333d5c8f7b46c2b51ff688daa6f43083c381d
# Generated: 2025-12-17T23:58:02+00:00
# Rust type: impact_physics::inertia::InertialProperties
# Type category: POD
# Commit: 7d41822d (dirty)
module [
    InertialProperties,
    write_bytes,
    from_bytes,
]

import Physics.InertiaTensor
import core.Builtin
import core.Point3

## The inertia-related properties of a physical body.
InertialProperties : {
    inertia_tensor : Physics.InertiaTensor.InertiaTensor,
    center_of_mass : Point3.Point3 Binary32,
    mass : F32,
}

## Serializes a value of [InertialProperties] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InertialProperties -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(88)
    |> Physics.InertiaTensor.write_bytes(value.inertia_tensor)
    |> Point3.write_bytes_32(value.center_of_mass)
    |> Builtin.write_bytes_f32(value.mass)

## Deserializes a value of [InertialProperties] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InertialProperties _
from_bytes = |bytes|
    Ok(
        {
            inertia_tensor: bytes |> List.sublist({ start: 0, len: 72 }) |> Physics.InertiaTensor.from_bytes?,
            center_of_mass: bytes |> List.sublist({ start: 72, len: 12 }) |> Point3.from_bytes_32?,
            mass: bytes |> List.sublist({ start: 84, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 88 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
