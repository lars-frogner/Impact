# Hash: 284e3605bb8d850c584b8443301efd8d136dea7a7411eb1a591bfd18d7dd5128
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact::physics::collision::CollidableID
# Type category: POD
# Commit: ce2d27b (dirty)
module [
    CollidableID,
    write_bytes,
    from_bytes,
]

import core.Builtin

## Identifier for a collidable in a [`CollisionWorld`].
CollidableID : U32

## Serializes a value of [CollidableID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CollidableID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_u32(value)

## Deserializes a value of [CollidableID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CollidableID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 4 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
