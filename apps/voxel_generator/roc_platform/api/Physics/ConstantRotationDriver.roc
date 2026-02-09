# Hash: c0df0b67eceef527
# Generated: 2026-02-09T21:22:24.23373697
# Rust type: impact_physics::driven_motion::constant_rotation::ConstantRotationDriver
# Type category: POD
module [
    ConstantRotationDriver,
    write_bytes,
    from_bytes,
]

import Entity
import Setup.ConstantRotation
import core.Builtin

## Driver for imposing constant rotation on a kinematic rigid body.
ConstantRotationDriver : {
    ## The entity being driven.
    entity_id : Entity.Id,
    ## The constant rotation imposed on the body.
    rotation : Setup.ConstantRotation.ConstantRotation,
    padding : F32,
}

## Serializes a value of [ConstantRotationDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantRotationDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> Entity.write_bytes_id(value.entity_id)
    |> Setup.ConstantRotation.write_bytes(value.rotation)
    |> Builtin.write_bytes_f32(value.padding)

## Deserializes a value of [ConstantRotationDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantRotationDriver _
from_bytes = |bytes|
    Ok(
        {
            entity_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
            rotation: bytes |> List.sublist({ start: 8, len: 36 }) |> Setup.ConstantRotation.from_bytes?,
            padding: bytes |> List.sublist({ start: 44, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 48 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
