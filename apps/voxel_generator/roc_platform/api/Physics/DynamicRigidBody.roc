# Hash: 3acc57f68bb196cb32f6fc3ffd21566f5914d742275c11421e4808193eec14cc
# Generated: 2025-12-17T23:54:08+00:00
# Rust type: impact_physics::rigid_body::DynamicRigidBody
# Type category: POD
# Commit: 7d41822d (dirty)
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
    position : Point3.Point3 Binary32,
    orientation : UnitQuaternion.UnitQuaternion Binary32,
    momentum : Vector3.Vector3 Binary32,
    angular_momentum : Vector3.Vector3 Binary32,
    total_force : Vector3.Vector3 Binary32,
    total_torque : Vector3.Vector3 Binary32,
}

## Serializes a value of [DynamicRigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicRigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(152)
    |> Builtin.write_bytes_f32(value.mass)
    |> Physics.InertiaTensor.write_bytes(value.inertia_tensor)
    |> Point3.write_bytes_32(value.position)
    |> UnitQuaternion.write_bytes_32(value.orientation)
    |> Vector3.write_bytes_32(value.momentum)
    |> Vector3.write_bytes_32(value.angular_momentum)
    |> Vector3.write_bytes_32(value.total_force)
    |> Vector3.write_bytes_32(value.total_torque)

## Deserializes a value of [DynamicRigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicRigidBody _
from_bytes = |bytes|
    Ok(
        {
            mass: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            inertia_tensor: bytes |> List.sublist({ start: 4, len: 72 }) |> Physics.InertiaTensor.from_bytes?,
            position: bytes |> List.sublist({ start: 76, len: 12 }) |> Point3.from_bytes_32?,
            orientation: bytes |> List.sublist({ start: 88, len: 16 }) |> UnitQuaternion.from_bytes_32?,
            momentum: bytes |> List.sublist({ start: 104, len: 12 }) |> Vector3.from_bytes_32?,
            angular_momentum: bytes |> List.sublist({ start: 116, len: 12 }) |> Vector3.from_bytes_32?,
            total_force: bytes |> List.sublist({ start: 128, len: 12 }) |> Vector3.from_bytes_32?,
            total_torque: bytes |> List.sublist({ start: 140, len: 12 }) |> Vector3.from_bytes_32?,
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
