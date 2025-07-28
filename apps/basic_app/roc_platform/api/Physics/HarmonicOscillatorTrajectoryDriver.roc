# Hash: 9c91002c32179ff701c9a4d316bd9fd3a498c005dd25a3bc2ee171c1ddc7e905
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_physics::driven_motion::harmonic_oscillation::HarmonicOscillatorTrajectoryDriver
# Type category: POD
# Commit: 397d36d3 (dirty)
module [
    HarmonicOscillatorTrajectoryDriver,
    write_bytes,
    from_bytes,
]

import Comp.KinematicRigidBodyID
import Setup.HarmonicOscillatorTrajectory

## Driver for imposing a harmonically oscillating trajectory on a kinematic
## rigid body.
HarmonicOscillatorTrajectoryDriver : {
    ## The kinematic rigid body being driven.
    rigid_body_id : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The harmonic oscillator trajectory imposed on the body.
    trajectory : Setup.HarmonicOscillatorTrajectory.HarmonicOscillatorTrajectory,
}

## Serializes a value of [HarmonicOscillatorTrajectoryDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, HarmonicOscillatorTrajectoryDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(80)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.HarmonicOscillatorTrajectory.write_bytes(value.trajectory)

## Deserializes a value of [HarmonicOscillatorTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result HarmonicOscillatorTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            trajectory: bytes |> List.sublist({ start: 8, len: 72 }) |> Setup.HarmonicOscillatorTrajectory.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 80 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
