# Hash: 8dfac041a23995970472776331a21e388896f8c6bcc5ec7003d8d8e415e467de
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::graph::CameraNodeID
# Type category: POD
# Commit: d505d37
module [
    CameraNodeID,
    write_bytes,
    from_bytes,
]

import Containers.GenerationalIdx

## Identifier for a [`CameraNode`] in a [`SceneGraph`].
CameraNodeID : Containers.GenerationalIdx.GenerationalIdx

## Serializes a value of [CameraNodeID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CameraNodeID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Containers.GenerationalIdx.write_bytes(value)

## Deserializes a value of [CameraNodeID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CameraNodeID _
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
