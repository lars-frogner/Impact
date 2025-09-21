# Hash: af437ed9cf19b46f8a42f5bbe25a363db3918b28b4af232058ee9cec13ea86b5
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_physics::rigid_body::KinematicRigidBody
# Type category: POD
# Commit: 397d36d3 (dirty)
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
    position : Point3.Point3 Binary64,
    orientation : UnitQuaternion.UnitQuaternion Binary64,
    velocity : Vector3.Vector3 Binary64,
    angular_velocity : Physics.AngularVelocity.AngularVelocity,
}

## Serializes a value of [KinematicRigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, KinematicRigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(112)
    |> Point3.write_bytes_64(value.position)
    |> UnitQuaternion.write_bytes_64(value.orientation)
    |> Vector3.write_bytes_64(value.velocity)
    |> Physics.AngularVelocity.write_bytes(value.angular_velocity)

## Deserializes a value of [KinematicRigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result KinematicRigidBody _
from_bytes = |bytes|
    Ok(
        {
            position: bytes |> List.sublist({ start: 0, len: 24 }) |> Point3.from_bytes_64?,
            orientation: bytes |> List.sublist({ start: 24, len: 32 }) |> UnitQuaternion.from_bytes_64?,
            velocity: bytes |> List.sublist({ start: 56, len: 24 }) |> Vector3.from_bytes_64?,
            angular_velocity: bytes |> List.sublist({ start: 80, len: 32 }) |> Physics.AngularVelocity.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 112 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
