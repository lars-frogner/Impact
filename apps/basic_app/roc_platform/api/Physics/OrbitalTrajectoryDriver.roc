# Hash: 825af81bc7f786c5
# Generated: 2026-02-09T21:21:57.029236554
# Rust type: impact_physics::driven_motion::orbit::OrbitalTrajectoryDriver
# Type category: POD
module [
    OrbitalTrajectoryDriver,
    write_bytes,
    from_bytes,
]

import Entity
import Setup.OrbitalTrajectory
import core.Builtin

## Driver for imposing an orbital trajectory on a kinematic rigid body.
OrbitalTrajectoryDriver : {
    ## The entity being driven.
    entity_id : Entity.Id,
    ## The orbital trajectory imposed on the body.
    trajectory : Setup.OrbitalTrajectory.OrbitalTrajectory,
    padding : F32,
}

## Serializes a value of [OrbitalTrajectoryDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, OrbitalTrajectoryDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Entity.write_bytes_id(value.entity_id)
    |> Setup.OrbitalTrajectory.write_bytes(value.trajectory)
    |> Builtin.write_bytes_f32(value.padding)

## Deserializes a value of [OrbitalTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result OrbitalTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            entity_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
            trajectory: bytes |> List.sublist({ start: 8, len: 44 }) |> Setup.OrbitalTrajectory.from_bytes?,
            padding: bytes |> List.sublist({ start: 52, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 56 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
