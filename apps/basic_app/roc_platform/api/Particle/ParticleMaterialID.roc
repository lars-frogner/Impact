# Hash: 319a33d1828ae973
# Generated: 2026-08-29T16:37:44.806067405
# Rust type: impact_particle::material::ParticleMaterialID
# Type category: POD
module [
    ParticleMaterialID,
    from_name,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for a material that determines the visual properties of a
## particle. Hashed from a material name.
ParticleMaterialID : Hashing.Hash32

## Converts the given material name into a [`ParticleMaterialID`].
from_name : Str -> ParticleMaterialID
from_name = |name|
    Hashing.hash_str_32(name)

## Serializes a value of [ParticleMaterialID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ParticleMaterialID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Hashing.write_bytes_hash_32(value)

## Deserializes a value of [ParticleMaterialID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ParticleMaterialID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Hashing.from_bytes_hash_32?,
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
