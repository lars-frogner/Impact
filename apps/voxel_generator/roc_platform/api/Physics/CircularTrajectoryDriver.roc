# Hash: 3a553c64273a6a33
# Generated: 2026-02-09T21:22:24.23373697
# Rust type: impact_physics::driven_motion::circular::CircularTrajectoryDriver
# Type category: POD
module [
    CircularTrajectoryDriver,
    write_bytes,
    from_bytes,
]

import Entity
import Setup.CircularTrajectory

## Driver for imposing a circular trajectory with constant speed on a kinematic
## rigid body.
CircularTrajectoryDriver : {
    ## The entity being driven.
    entity_id : Entity.Id,
    ## The circular trajectory imposed on the body.
    trajectory : Setup.CircularTrajectory.CircularTrajectory,
}

## Serializes a value of [CircularTrajectoryDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CircularTrajectoryDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> Entity.write_bytes_id(value.entity_id)
    |> Setup.CircularTrajectory.write_bytes(value.trajectory)

## Deserializes a value of [CircularTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CircularTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            entity_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
            trajectory: bytes |> List.sublist({ start: 8, len: 40 }) |> Setup.CircularTrajectory.from_bytes?,
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
