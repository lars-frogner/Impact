# Hash: 3e989192f076d2bc30ae6a3a433da81e47a154b580ff29a0ec1057e685da99e2
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact::command::rendering::postprocessing::ToExposure
# Type category: Inline
# Commit: 397d36d3 (dirty)
module [
    ToExposure,
    write_bytes,
    from_bytes,
]

import core.Builtin

ToExposure : [
    DifferentByStops F32,
    Auto {
            ev_compensation : F32,
        },
    Manual {
            iso : F32,
        },
]

## Serializes a value of [ToExposure] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToExposure -> List U8
write_bytes = |bytes, value|
    when value is
        DifferentByStops(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(0)
            |> Builtin.write_bytes_f32(val)

        Auto { ev_compensation } ->
            bytes
            |> List.reserve(5)
            |> List.append(1)
            |> Builtin.write_bytes_f32(ev_compensation)

        Manual { iso } ->
            bytes
            |> List.reserve(5)
            |> List.append(2)
            |> Builtin.write_bytes_f32(iso)

## Deserializes a value of [ToExposure] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToExposure _
from_bytes = |bytes|
    if List.len(bytes) != 5 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    DifferentByStops(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    Auto     {
                        ev_compensation: data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    },
                )


            [2, .. as data_bytes] ->
                Ok(
                    Manual     {
                        iso: data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    },
                )


            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
