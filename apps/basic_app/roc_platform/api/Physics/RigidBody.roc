# Hash: 5d1b49cb8bc50cdd351d5818954e12ea41c15a363dfc842e8f0a28e059434ce4
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::physics::rigid_body::RigidBody
# Type category: POD
# Commit: 31f3514 (dirty)
module [
    RigidBody,
    write_bytes,
    from_bytes,
]

import Physics.InertialProperties
import core.Vector3

## A rigid body. It holds its [`InertialProperties`], the total [`Force`] and
## [`Torque`] it is subjected to as well as its [`Momentum`] and
## [`AngularMomentum`]. To avoid replication of data, the body does not store
## or manage its position, orientation, velocity and angular velocity. The
## reason it stores its linear and angular momentum is that these are the
## conserved quantities in free motion and thus should be the primary
## variables in the simulation, with linear and angular velocity being derived
## from them. This means that the body's linear or angular momentum has to be
## updated whenever something manually modifies the linear or angular
## velocity, respectively.
RigidBody : {
    inertial_properties : Physics.InertialProperties.InertialProperties,
    momentum : Vector3.Vector3 Binary64,
    angular_momentum : Vector3.Vector3 Binary64,
    total_force : Vector3.Vector3 Binary64,
    total_torque : Vector3.Vector3 Binary64,
}

## Serializes a value of [RigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(272)
    |> Physics.InertialProperties.write_bytes(value.inertial_properties)
    |> Vector3.write_bytes_64(value.momentum)
    |> Vector3.write_bytes_64(value.angular_momentum)
    |> Vector3.write_bytes_64(value.total_force)
    |> Vector3.write_bytes_64(value.total_torque)

## Deserializes a value of [RigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RigidBody _
from_bytes = |bytes|
    Ok(
        {
            inertial_properties: bytes |> List.sublist({ start: 0, len: 176 }) |> Physics.InertialProperties.from_bytes?,
            momentum: bytes |> List.sublist({ start: 176, len: 24 }) |> Vector3.from_bytes_64?,
            angular_momentum: bytes |> List.sublist({ start: 200, len: 24 }) |> Vector3.from_bytes_64?,
            total_force: bytes |> List.sublist({ start: 224, len: 24 }) |> Vector3.from_bytes_64?,
            total_torque: bytes |> List.sublist({ start: 248, len: 24 }) |> Vector3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 272 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
