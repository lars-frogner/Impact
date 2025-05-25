# Hash: 95af002dc6d49d1521b98a4ce1c1a102815a7f37e228ea98eabcff124cbf47a6
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::scene::graph::GroupNodeID
# Type category: POD
# Commit: 31f3514 (dirty)
module [
    GroupNodeID,
    write_bytes,
    from_bytes,
]

import Containers.GenerationalIdx

## Identifier for a [`GroupNode`] in a [`SceneGraph`].
GroupNodeID : Containers.GenerationalIdx.GenerationalIdx

## Serializes a value of [GroupNodeID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GroupNodeID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Containers.GenerationalIdx.write_bytes(value)

## Deserializes a value of [GroupNodeID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GroupNodeID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 16 }) |> Containers.GenerationalIdx.from_bytes?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
