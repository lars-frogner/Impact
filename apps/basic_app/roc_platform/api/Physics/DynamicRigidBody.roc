# Hash: a07dbe6aeed3f1d1835cf0bbcb9f65e61caea333e16d7e25757e6793f496c206
# Generated: 2025-12-21T22:57:59+00:00
# Rust type: impact_physics::rigid_body::DynamicRigidBody
# Type category: POD
# Commit: d4c84c05 (dirty)
module [
    DynamicRigidBody,
    write_bytes,
    from_bytes,
]

import Physics.InertiaTensor
import core.Builtin
import core.Point3
import core.UnitQuaternion
import core.Vector3

## A rigid body whose motion is affected by the force and torque it experiences
## as well as its inertial properties.
##
## The body stores its linear and angular momentum rather than its linear and
## angular velocity. The reason for this is that these are the conserved
## quantities in free motion and thus should be the primary variables in the
## simulation, with linear and angular velocity being derived from them (even
## when left to rotate freely without torqe, the angular velocity will change
## over time, while the angular momentum stays constant). This means that the
## body's linear or angular momentum has to be updated whenever something
## manually modifies the linear or angular velocity, respectively.
DynamicRigidBody : {
    mass : F32,
    inertia_tensor : Physics.InertiaTensor.InertiaTensor,
    position : Point3.Point3,
    orientation : UnitQuaternion.UnitQuaternion,
    momentum : Vector3.Vector3,
    angular_momentum : Vector3.Vector3,
    total_force : Vector3.Vector3,
    total_torque : Vector3.Vector3,
}

## Serializes a value of [DynamicRigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicRigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(152)
    |> Builtin.write_bytes_f32(value.mass)
    |> Physics.InertiaTensor.write_bytes(value.inertia_tensor)
    |> Point3.write_bytes(value.position)
    |> UnitQuaternion.write_bytes(value.orientation)
    |> Vector3.write_bytes(value.momentum)
    |> Vector3.write_bytes(value.angular_momentum)
    |> Vector3.write_bytes(value.total_force)
    |> Vector3.write_bytes(value.total_torque)

## Deserializes a value of [DynamicRigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicRigidBody _
from_bytes = |bytes|
    Ok(
        {
            mass: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            inertia_tensor: bytes |> List.sublist({ start: 4, len: 72 }) |> Physics.InertiaTensor.from_bytes?,
            position: bytes |> List.sublist({ start: 76, len: 12 }) |> Point3.from_bytes?,
            orientation: bytes |> List.sublist({ start: 88, len: 16 }) |> UnitQuaternion.from_bytes?,
            momentum: bytes |> List.sublist({ start: 104, len: 12 }) |> Vector3.from_bytes?,
            angular_momentum: bytes |> List.sublist({ start: 116, len: 12 }) |> Vector3.from_bytes?,
            total_force: bytes |> List.sublist({ start: 128, len: 12 }) |> Vector3.from_bytes?,
            total_torque: bytes |> List.sublist({ start: 140, len: 12 }) |> Vector3.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 152 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
