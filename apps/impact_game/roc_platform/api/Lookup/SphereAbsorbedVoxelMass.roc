# Hash: cca5e8306d12a324
# Generated: 2026-01-31T22:02:32.218650242
# Rust type: impact_game::player::tools::SphereAbsorbedVoxelMass
# Type category: POD
module [
    SphereAbsorbedVoxelMass,
    get!,
    write_bytes,
    from_bytes,
]

import Entity
import Lookup.GameLookupTarget
import core.Builtin

SphereAbsorbedVoxelMass : {
    mass : F32,
}

## Fetch the current value of this quantity.
get! : Entity.Id => Result SphereAbsorbedVoxelMass Str
get! = |entity_id|
    Lookup.GameLookupTarget.lookup!(SphereAbsorbedVoxelMass{entity_id, }, from_bytes)

## Serializes a value of [SphereAbsorbedVoxelMass] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SphereAbsorbedVoxelMass -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_f32(value.mass)

## Deserializes a value of [SphereAbsorbedVoxelMass] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SphereAbsorbedVoxelMass _
from_bytes = |bytes|
    Ok(
        {
            mass: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
        },
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
