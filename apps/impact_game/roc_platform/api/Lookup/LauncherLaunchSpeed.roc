# Hash: e7c9bd8364b73e49
# Generated: 2026-02-01T20:43:19.603855004
# Rust type: impact_game::player::tools::LauncherLaunchSpeed
# Type category: POD
module [
    LauncherLaunchSpeed,
    get!,
    write_bytes,
    from_bytes,
]

import Lookup.GameLookupTarget
import core.Builtin

LauncherLaunchSpeed : {
    speed : F32,
}

## Fetch the current value of this quantity.
get! : {} => Result LauncherLaunchSpeed Str
get! = |{}|
    Lookup.GameLookupTarget.lookup!(LauncherLaunchSpeed, from_bytes)

## Serializes a value of [LauncherLaunchSpeed] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LauncherLaunchSpeed -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_f32(value.speed)

## Deserializes a value of [LauncherLaunchSpeed] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LauncherLaunchSpeed _
from_bytes = |bytes|
    Ok(
        {
            speed: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
        },
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
