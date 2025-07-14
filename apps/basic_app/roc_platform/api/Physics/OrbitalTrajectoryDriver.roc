# Hash: 5e4151a3d461babb97e05796effeaebc3d3da96345598e9b3bc43b754eeb1415
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_physics::driven_motion::orbit::OrbitalTrajectoryDriver
# Type category: POD
# Commit: b1b4dfd8 (dirty)
module [
    OrbitalTrajectoryDriver,
    write_bytes,
    from_bytes,
]

import Comp.KinematicRigidBodyID
import Setup.OrbitalTrajectory

## Driver for imposing an orbital trajectory on a kinematic rigid body.
OrbitalTrajectoryDriver : {
    ## The kinematic rigid body being driven.
    rigid_body_id : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The orbital trajectory imposed on the body.
    trajectory : Setup.OrbitalTrajectory.OrbitalTrajectory,
}

## Serializes a value of [OrbitalTrajectoryDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, OrbitalTrajectoryDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(96)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.OrbitalTrajectory.write_bytes(value.trajectory)

## Deserializes a value of [OrbitalTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result OrbitalTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            trajectory: bytes |> List.sublist({ start: 8, len: 88 }) |> Setup.OrbitalTrajectory.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 96 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
