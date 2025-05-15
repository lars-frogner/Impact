# Hash: 6705a5a7de26cec6ea7f02f77b4a1ffca89bc8b72bdcfc2ca5a7de219aeb54c2
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::command::ToSimulationSpeedMultiplier
# Type category: Inline
# Commit: d505d37
module [
    ToSimulationSpeedMultiplier,
    write_bytes,
    from_bytes,
]

import core.Builtin

ToSimulationSpeedMultiplier : [
    Higher,
    Lower,
    Specific F64,
]

## Serializes a value of [ToSimulationSpeedMultiplier] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToSimulationSpeedMultiplier -> List U8
write_bytes = |bytes, value|
    when value is
        Higher ->
            bytes
            |> List.reserve(9)
            |> List.append(0)
            |> List.concat(List.repeat(0, 8))

        Lower ->
            bytes
            |> List.reserve(9)
            |> List.append(1)
            |> List.concat(List.repeat(0, 8))

        Specific(val) ->
            bytes
            |> List.reserve(9)
            |> List.append(2)
            |> Builtin.write_bytes_f64(val)

## Deserializes a value of [ToSimulationSpeedMultiplier] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToSimulationSpeedMultiplier _
from_bytes = |bytes|
    if List.len(bytes) != 9 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Higher)
            [1, ..] -> Ok(Lower)
            [2, .. as data_bytes] ->
                Ok(
                    Specific(
                        data_bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 8)?
    test_roundtrip_for_variant(1, 1, 8)?
    test_roundtrip_for_variant(2, 9, 0)?
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
