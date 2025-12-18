# Hash: 2e2f38ad45d965b7d007ec8c63e6d5a896c0a31cea3a72e13b3be671236673b3
# Generated: 2025-12-17T23:54:08+00:00
# Rust type: impact_physics::rigid_body::KinematicRigidBody
# Type category: POD
# Commit: 7d41822d (dirty)
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
    orientation : UnitQuaternion.UnitQuaternion Binary32,
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
    |> UnitQuaternion.write_bytes_32(value.orientation)
    |> Vector3.write_bytes_32(value.velocity)
    |> Physics.AngularVelocity.write_bytes(value.angular_velocity)

## Deserializes a value of [KinematicRigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result KinematicRigidBody _
from_bytes = |bytes|
    Ok(
        {
            position: bytes |> List.sublist({ start: 0, len: 12 }) |> Point3.from_bytes_32?,
            orientation: bytes |> List.sublist({ start: 12, len: 16 }) |> UnitQuaternion.from_bytes_32?,
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
