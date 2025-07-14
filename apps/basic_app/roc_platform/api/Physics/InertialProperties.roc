# Hash: f4f8e4ec51b085f1884aedba55fd61f2eea4a2b1b61ecfede8052f7394567f7b
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_physics::inertia::InertialProperties
# Type category: POD
# Commit: b1b4dfd8 (dirty)
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
    center_of_mass : Point3.Point3 Binary64,
    mass : F64,
}

## Serializes a value of [InertialProperties] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InertialProperties -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(176)
    |> Physics.InertiaTensor.write_bytes(value.inertia_tensor)
    |> Point3.write_bytes_64(value.center_of_mass)
    |> Builtin.write_bytes_f64(value.mass)

## Deserializes a value of [InertialProperties] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InertialProperties _
from_bytes = |bytes|
    Ok(
        {
            inertia_tensor: bytes |> List.sublist({ start: 0, len: 144 }) |> Physics.InertiaTensor.from_bytes?,
            center_of_mass: bytes |> List.sublist({ start: 144, len: 24 }) |> Point3.from_bytes_64?,
            mass: bytes |> List.sublist({ start: 168, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 176 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
