# Hash: 1a9a749108939760
# Generated: 2026-01-01T09:41:16.584947407
# Rust type: impact_containers::slot_map::SlotKey
# Type category: POD
module [
    SlotKey,
    write_bytes,
    from_bytes,
]

import Containers.Generation
import core.Builtin

## A key into a [`SlotMap`].
SlotKey : {
    generation : Containers.Generation.Generation,
    idx : U32,
}

## Serializes a value of [SlotKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SlotKey -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Containers.Generation.write_bytes(value.generation)
    |> Builtin.write_bytes_u32(value.idx)

## Deserializes a value of [SlotKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SlotKey _
from_bytes = |bytes|
    Ok(
        {
            generation: bytes |> List.sublist({ start: 0, len: 4 }) |> Containers.Generation.from_bytes?,
            idx: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_u32?,
        },
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
