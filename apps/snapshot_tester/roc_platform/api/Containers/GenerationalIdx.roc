# Hash: 324672bef4540033796d404df5fa910c160854b2fba9371fb5dc69af38ab69ea
# Generated: 2025-07-13T20:18:37+00:00
# Rust type: impact_containers::generational_reusing_vec::GenerationalIdx
# Type category: POD
# Commit: b1b4dfd8 (dirty)
module [
    GenerationalIdx,
    write_bytes,
    from_bytes,
]

import core.NativeNum

## An index into a [`GenerationalReusingVec`].
GenerationalIdx : {
    generation : NativeNum.Usize,
    idx : NativeNum.Usize,
}

## Serializes a value of [GenerationalIdx] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GenerationalIdx -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> NativeNum.write_bytes_usize(value.generation)
    |> NativeNum.write_bytes_usize(value.idx)

## Deserializes a value of [GenerationalIdx] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GenerationalIdx _
from_bytes = |bytes|
    Ok(
        {
            generation: bytes |> List.sublist({ start: 0, len: 8 }) |> NativeNum.from_bytes_usize?,
            idx: bytes |> List.sublist({ start: 8, len: 8 }) |> NativeNum.from_bytes_usize?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
