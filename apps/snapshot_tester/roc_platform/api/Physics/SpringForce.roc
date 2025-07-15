# Hash: 4e6cde0495b430e8b46a70f3408a572fde116af7aa8101194490ba0be55fbb1a
# Generated: 2025-07-15T11:05:49+00:00
# Rust type: impact_physics::force::spring_force::SpringForce
# Type category: POD
# Commit: 189570ab (dirty)
module [
    SpringForce,
    new,
    write_bytes,
    from_bytes,
]

import Physics.Spring
import core.Point3

## A spring force between to rigid bodies.
SpringForce : {
    ## The spring connecting the bodies.
    spring : Physics.Spring.Spring,
    ## The point where the spring is attached to the first body, in that
    ## body's local reference frame.
    attachment_point_1 : Point3.Point3 Binary64,
    ## The point where the spring is attached to the second body, in that
    ## body's local reference frame.
    attachment_point_2 : Point3.Point3 Binary64,
}

## Defines the force from the given string between the given attachment
## points in their respective body's reference frame.
new : Physics.Spring.Spring, Point3.Point3 Binary64, Point3.Point3 Binary64 -> SpringForce
new = |spring, attachment_point_1, attachment_point_2|
    {
        spring,
        attachment_point_1,
        attachment_point_2,
    }

## Serializes a value of [SpringForce] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SpringForce -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(80)
    |> Physics.Spring.write_bytes(value.spring)
    |> Point3.write_bytes_64(value.attachment_point_1)
    |> Point3.write_bytes_64(value.attachment_point_2)

## Deserializes a value of [SpringForce] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SpringForce _
from_bytes = |bytes|
    Ok(
        {
            spring: bytes |> List.sublist({ start: 0, len: 32 }) |> Physics.Spring.from_bytes?,
            attachment_point_1: bytes |> List.sublist({ start: 32, len: 24 }) |> Point3.from_bytes_64?,
            attachment_point_2: bytes |> List.sublist({ start: 56, len: 24 }) |> Point3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 80 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
