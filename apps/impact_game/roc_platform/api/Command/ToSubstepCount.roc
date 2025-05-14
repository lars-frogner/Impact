# Hash: 06ea5345eb96f2e47df8fa815b5a8d0b4c0d531c1d4e913385e8ba7a131d54dd
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::command::ToSubstepCount
# Type category: Inline
# Commit: d505d37
module [
    ToSubstepCount,
    write_bytes,
    from_bytes,
]

import core.Builtin

ToSubstepCount : [
    HigherBy U32,
    LowerBy U32,
    Specific U32,
]

## Serializes a value of [ToSubstepCount] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToSubstepCount -> List U8
write_bytes = |bytes, value|
    when value is
        HigherBy(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(0)
            |> Builtin.write_bytes_u32(val)

        LowerBy(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(1)
            |> Builtin.write_bytes_u32(val)

        Specific(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(2)
            |> Builtin.write_bytes_u32(val)

## Deserializes a value of [ToSubstepCount] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToSubstepCount _
from_bytes = |bytes|
    if List.len(bytes) != 5 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    HigherBy(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    LowerBy(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    Specific(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 5, 0)?
    test_roundtrip_for_variant(1, 5, 0)?
    test_roundtrip_for_variant(2, 5, 0)?
    Ok({})

test_roundtrip_for_variant : U8, U64, U64 -> Result {} _
test_roundtrip_for_variant = |discriminant, variant_size, padding_size|
    bytes = 
        List.range({ start: At discriminant, end: Length variant_size })
        |> List.concat(List.repeat(0, padding_size))
        |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
