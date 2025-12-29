# Hash: 7f71c17ef6ec6ff4
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_physics::driven_motion::harmonic_oscillation::HarmonicOscillatorTrajectoryDriver
# Type category: POD
module [
    HarmonicOscillatorTrajectoryDriver,
    write_bytes,
    from_bytes,
]

import Comp.KinematicRigidBodyID
import Setup.HarmonicOscillatorTrajectory
import core.Builtin

## Driver for imposing a harmonically oscillating trajectory on a kinematic
## rigid body.
HarmonicOscillatorTrajectoryDriver : {
    ## The kinematic rigid body being driven.
    rigid_body_id : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The harmonic oscillator trajectory imposed on the body.
    trajectory : Setup.HarmonicOscillatorTrajectory.HarmonicOscillatorTrajectory,
    padding : F32,
}

## Serializes a value of [HarmonicOscillatorTrajectoryDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, HarmonicOscillatorTrajectoryDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.HarmonicOscillatorTrajectory.write_bytes(value.trajectory)
    |> Builtin.write_bytes_f32(value.padding)

## Deserializes a value of [HarmonicOscillatorTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result HarmonicOscillatorTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            trajectory: bytes |> List.sublist({ start: 8, len: 36 }) |> Setup.HarmonicOscillatorTrajectory.from_bytes?,
            padding: bytes |> List.sublist({ start: 44, len: 4 }) |> Builtin.from_bytes_f32?,
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
