# Hash: 499212dad22e0951c7584127214c3442b88b88431454063237bef474931b837e
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::rendering::postprocessing::capturing::SensorSensitivity
# Type category: Inline
# Commit: d505d37
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
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 5, 0)?
    test_roundtrip_for_variant(1, 5, 0)?
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
