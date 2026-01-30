# Hash: cef2ece1cf2dc5ae
# Generated: 2026-01-30T13:59:44.560028361
# Rust type: impact_game::lookup::InventoryMass
# Type category: POD
module [
    InventoryMass,
    get!,
    write_bytes,
    from_bytes,
]

import Lookup.GameLookupTarget
import core.Builtin

InventoryMass : {
    mass : F32,
}

## Fetch the current value of this quantity.
get! : {} => Result InventoryMass Str
get! = |{}|
    Lookup.GameLookupTarget.lookup!(InventoryMass, from_bytes)

## Serializes a value of [InventoryMass] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InventoryMass -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_f32(value.mass)

## Deserializes a value of [InventoryMass] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InventoryMass _
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
