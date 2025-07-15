# Hash: edda419730672a6cfe46906b94ae4b48018486a767ca5662a2b2a35120748801
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact_scene::graph::ModelInstanceNodeID
# Type category: POD
# Commit: 189570ab (dirty)
module [
    ModelInstanceNodeID,
    write_bytes,
    from_bytes,
]

import Containers.GenerationalIdx

## Identifier for a [`ModelInstanceNode`] in a [`SceneGraph`].
ModelInstanceNodeID : Containers.GenerationalIdx.GenerationalIdx

## Serializes a value of [ModelInstanceNodeID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ModelInstanceNodeID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Containers.GenerationalIdx.write_bytes(value)

## Deserializes a value of [ModelInstanceNodeID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ModelInstanceNodeID _
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
