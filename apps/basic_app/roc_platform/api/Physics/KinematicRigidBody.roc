# Hash: 67c4c4cde3c60f72c3e33a9929dbb417dda9165b0353991acc43d8f1b2db97fa
# Generated: 2025-12-20T20:26:51+00:00
# Rust type: impact_physics::rigid_body::KinematicRigidBody
# Type category: POD
# Commit: dbb3dcfa (dirty)
module [
    KinematicRigidBody,
    write_bytes,
    from_bytes,
]

import Physics.AngularVelocity
import core.Point3
import core.UnitQuaternion
import core.Vector3

## A rigid body whose linear and angular velocity only change when explicitly
## modified. It does not have any inertial properties, and is not affected by
## forces or torques.
KinematicRigidBody : {
    position : Point3.Point3 Binary32,
    orientation : UnitQuaternion.UnitQuaternion,
    velocity : Vector3.Vector3 Binary32,
    angular_velocity : Physics.AngularVelocity.AngularVelocity,
}

## Serializes a value of [KinematicRigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, KinematicRigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Point3.write_bytes_32(value.position)
    |> UnitQuaternion.write_bytes(value.orientation)
    |> Vector3.write_bytes_32(value.velocity)
    |> Physics.AngularVelocity.write_bytes(value.angular_velocity)

## Deserializes a value of [KinematicRigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result KinematicRigidBody _
from_bytes = |bytes|
    Ok(
        {
            position: bytes |> List.sublist({ start: 0, len: 12 }) |> Point3.from_bytes_32?,
            orientation: bytes |> List.sublist({ start: 12, len: 16 }) |> UnitQuaternion.from_bytes?,
            velocity: bytes |> List.sublist({ start: 28, len: 12 }) |> Vector3.from_bytes_32?,
            angular_velocity: bytes |> List.sublist({ start: 40, len: 16 }) |> Physics.AngularVelocity.from_bytes?,
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
