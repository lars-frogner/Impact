# Hash: f063ca928a3ada7c06da425db8a707367d99bc7482fdc58e326438e6aa2753d0
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_scene::graph::CameraNodeID
# Type category: POD
# Commit: 397d36d3 (dirty)
module [
    CameraNodeID,
    write_bytes,
    from_bytes,
]

import Containers.SlotKey

## Identifier for a [`CameraNode`] in a [`SceneGraph`].
CameraNodeID : Containers.SlotKey.SlotKey

## Serializes a value of [CameraNodeID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CameraNodeID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Containers.SlotKey.write_bytes(value)

## Deserializes a value of [CameraNodeID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CameraNodeID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Containers.SlotKey.from_bytes?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 8 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
