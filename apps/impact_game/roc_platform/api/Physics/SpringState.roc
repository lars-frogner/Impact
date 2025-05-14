# Hash: 171b2427cd33ab50d693bb5c45e552595a50e7f28daad09ac27ee55312230323
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::rigid_body::forces::spring::SpringState
# Type category: POD
# Commit: d505d37
module [
    SpringState,
    new,
    write_bytes,
    from_bytes,
]

import core.Point3
import core.UnitVector3

## The current state of a spring.
SpringState : {
    ## The direction from the first to the second attachment point.
    direction : UnitVector3.UnitVector3 Binary64,
    ## The position of the center of the spring.
    center : Point3.Point3 Binary64,
}

## Creates a new spring state (with dummy values).
new : {} -> SpringState
new = |{}|
    {
        direction: UnitVector3.y_axis,
        center: Point3.origin,
    }

## Serializes a value of [SpringState] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SpringState -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> UnitVector3.write_bytes_64(value.direction)
    |> Point3.write_bytes_64(value.center)

## Deserializes a value of [SpringState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SpringState _
from_bytes = |bytes|
    Ok(
        {
            direction: bytes |> List.sublist({ start: 0, len: 24 }) |> UnitVector3.from_bytes_64?,
            center: bytes |> List.sublist({ start: 24, len: 24 }) |> Point3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 48 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
