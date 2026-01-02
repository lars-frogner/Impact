# Hash: b585567f02971947
# Generated: 2026-01-01T09:41:16.584947407
# Rust type: impact_model::InstanceFeatureTypeID
# Type category: POD
module [
    InstanceFeatureTypeID,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for a type of instance feature.
InstanceFeatureTypeID : Hashing.Hash64

## Serializes a value of [InstanceFeatureTypeID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InstanceFeatureTypeID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_hash_64(value)

## Deserializes a value of [InstanceFeatureTypeID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InstanceFeatureTypeID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Hashing.from_bytes_hash_64?,
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
