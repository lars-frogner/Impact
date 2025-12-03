# Hash: f549c7c6ec76ee5cf11c6affd8871bbaefa91a32a770302833f2d32ec9cb8ddc
# Generated: 2025-12-03T23:15:27+00:00
# Rust type: impact_voxel::generation::VoxelGeneratorID
# Type category: POD
# Commit: b393e25e (dirty)
module [
    VoxelGeneratorID,
    from_name,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for a voxel generator.
VoxelGeneratorID : Hashing.StringHash64

## Creates a voxel generator ID hashed from the given name.
from_name : Str -> VoxelGeneratorID
from_name = |name|
    Hashing.hash_str_64(name)

## Serializes a value of [VoxelGeneratorID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelGeneratorID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_string_hash_64(value)

## Deserializes a value of [VoxelGeneratorID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelGeneratorID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Hashing.from_bytes_string_hash_64?,
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
