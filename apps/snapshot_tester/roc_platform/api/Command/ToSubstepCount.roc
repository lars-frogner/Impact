# Hash: 567bf1473951aadcbe5b097bb68a3d03e25ce65421ef51e1af74e4d569aa9f3f
# Generated: 2025-07-15T17:32:43+00:00
# Rust type: impact::physics::command::ToSubstepCount
# Type category: Inline
# Commit: 1fbb6f6b (dirty)
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
