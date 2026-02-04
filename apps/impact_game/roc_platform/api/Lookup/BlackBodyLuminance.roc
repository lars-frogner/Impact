# Hash: e866083fd9833dd7
# Generated: 2026-02-02T22:19:46.27680873
# Rust type: impact_game::entities::black_body::BlackBodyLuminance
# Type category: POD
module [
    BlackBodyLuminance,
    get!,
    write_bytes,
    from_bytes,
]

import Lookup.GameLookupTarget
import core.Builtin
import core.Vector3

BlackBodyLuminance : {
    rgb_luminance : Vector3.Vector3,
    total_luminance : F32,
}

## Fetch the current value of this quantity.
get! : F32 => Result BlackBodyLuminance Str
get! = |temperature|
    Lookup.GameLookupTarget.lookup!(BlackBodyLuminance{temperature, }, from_bytes)

## Serializes a value of [BlackBodyLuminance] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, BlackBodyLuminance -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Vector3.write_bytes(value.rgb_luminance)
    |> Builtin.write_bytes_f32(value.total_luminance)

## Deserializes a value of [BlackBodyLuminance] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result BlackBodyLuminance _
from_bytes = |bytes|
    Ok(
        {
            rgb_luminance: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes?,
            total_luminance: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
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
