# Hash: a289e7331c31b7180d797f89b4cb8cbf84c3980e0e7214615e9c418c09905022
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::voxel::voxel_types::VoxelType
# Type category: POD
# Commit: d505d37
module [
    VoxelType,
    write_bytes,
    from_bytes,
]

import core.Builtin

## A type identifier that determines all the properties of a voxel.
VoxelType : U8

## Serializes a value of [VoxelType] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelType -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(1)
    |> Builtin.write_bytes_u8(value)

## Deserializes a value of [VoxelType] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelType _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 1 }) |> Builtin.from_bytes_u8?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 1 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
