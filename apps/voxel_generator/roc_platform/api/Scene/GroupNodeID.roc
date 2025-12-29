# Hash: af2daeb0ed08ddd0
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_scene::graph::GroupNodeID
# Type category: POD
module [
    GroupNodeID,
    write_bytes,
    from_bytes,
]

import Containers.SlotKey

## Identifier for a group node in a [`SceneGraph`].
GroupNodeID : Containers.SlotKey.SlotKey

## Serializes a value of [GroupNodeID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GroupNodeID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Containers.SlotKey.write_bytes(value)

## Deserializes a value of [GroupNodeID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GroupNodeID _
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
