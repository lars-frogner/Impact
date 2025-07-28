# Hash: e3be272194ebec8e785ffd73b3d5ab89e46ded76bb5d13a90a7c78a1f64cfdbf
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_physics::rigid_body::DynamicRigidBody
# Type category: POD
# Commit: 397d36d3 (dirty)
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
    mass : F64,
    inertia_tensor : Physics.InertiaTensor.InertiaTensor,
    position : Point3.Point3 Binary64,
    orientation : UnitQuaternion.UnitQuaternion Binary64,
    momentum : Vector3.Vector3 Binary64,
    angular_momentum : Vector3.Vector3 Binary64,
    total_force : Vector3.Vector3 Binary64,
    total_torque : Vector3.Vector3 Binary64,
}

## Serializes a value of [DynamicRigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicRigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(304)
    |> Builtin.write_bytes_f64(value.mass)
    |> Physics.InertiaTensor.write_bytes(value.inertia_tensor)
    |> Point3.write_bytes_64(value.position)
    |> UnitQuaternion.write_bytes_64(value.orientation)
    |> Vector3.write_bytes_64(value.momentum)
    |> Vector3.write_bytes_64(value.angular_momentum)
    |> Vector3.write_bytes_64(value.total_force)
    |> Vector3.write_bytes_64(value.total_torque)

## Deserializes a value of [DynamicRigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicRigidBody _
from_bytes = |bytes|
    Ok(
        {
            mass: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            inertia_tensor: bytes |> List.sublist({ start: 8, len: 144 }) |> Physics.InertiaTensor.from_bytes?,
            position: bytes |> List.sublist({ start: 152, len: 24 }) |> Point3.from_bytes_64?,
            orientation: bytes |> List.sublist({ start: 176, len: 32 }) |> UnitQuaternion.from_bytes_64?,
            momentum: bytes |> List.sublist({ start: 208, len: 24 }) |> Vector3.from_bytes_64?,
            angular_momentum: bytes |> List.sublist({ start: 232, len: 24 }) |> Vector3.from_bytes_64?,
            total_force: bytes |> List.sublist({ start: 256, len: 24 }) |> Vector3.from_bytes_64?,
            total_torque: bytes |> List.sublist({ start: 280, len: 24 }) |> Vector3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 304 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
