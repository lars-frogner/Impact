# Hash: cf81b690bc0dbb07
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_containers::slot_map::Generation
# Type category: POD
module [
    Generation,
    write_bytes,
    from_bytes,
]

import core.Builtin

Generation : U32

## Serializes a value of [Generation] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Generation -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_u32(value)

## Deserializes a value of [Generation] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Generation _
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
