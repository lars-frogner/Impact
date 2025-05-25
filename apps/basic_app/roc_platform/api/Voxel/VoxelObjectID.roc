# Hash: 36dfa5d37287875ee7369d49c0de834ae97cd218ac6b214c184f343ebf2e86db
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::voxel::VoxelObjectID
# Type category: POD
# Commit: 31f3514 (dirty)
module [
    VoxelObjectID,
    write_bytes,
    from_bytes,
]

import core.Builtin

## Identifier for a [`ChunkedVoxelObject`] in a [`VoxelManager`].
VoxelObjectID : U32

## Serializes a value of [VoxelObjectID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelObjectID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_u32(value)

## Deserializes a value of [VoxelObjectID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelObjectID _
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
