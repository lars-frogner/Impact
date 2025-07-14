# Hash: 7c0fdec79dc5595cfbe6dce642e8909904bdb332f4a6310450f9aa60a2c1d580
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_rendering::postprocessing::capturing::SensorSensitivity
# Type category: Inline
# Commit: b1b4dfd8 (dirty)
module [
    SensorSensitivity,
    write_bytes,
    from_bytes,
]

import core.Builtin

## The sensitivity of a camera sensor, which may be set manually as an ISO
## value or determined automatically based on the incident luminance, with
## optional exposure value compensation in f-stops.
SensorSensitivity : [
    Manual {
            iso : F32,
        },
    Auto {
            ev_compensation : F32,
        },
]

## Serializes a value of [SensorSensitivity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SensorSensitivity -> List U8
write_bytes = |bytes, value|
    when value is
        Manual { iso } ->
            bytes
            |> List.reserve(5)
            |> List.append(0)
            |> Builtin.write_bytes_f32(iso)

        Auto { ev_compensation } ->
            bytes
            |> List.reserve(5)
            |> List.append(1)
            |> Builtin.write_bytes_f32(ev_compensation)

## Deserializes a value of [SensorSensitivity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SensorSensitivity _
from_bytes = |bytes|
    if List.len(bytes) != 5 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    Manual     {
                        iso: data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    },
                )

            [1, .. as data_bytes] ->
                Ok(
                    Auto     {
                        ev_compensation: data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    },
                )


            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
