# Hash: ef570265ff5906352971853a8556cbd527e99ecca17a4deae4b885a3fd2cbde0
# Generated: 2025-12-28T19:52:23+00:00
# Rust type: impact_physics::quantities::AngularVelocity
# Type category: POD
# Commit: 78e3beb5 (dirty)
module [
    AngularVelocity,
    new,
    from_vector,
    zero,
    write_bytes,
    from_bytes,
]

import core.Radians
import core.UnitVector3
import core.Vector3

## An angular velocity in 3D space, represented by an axis of rotation and an
## angular speed.
##
## This type is primarily intended for compact storage inside other types and
## collections. For computations, prefer the SIMD-friendly 16-byte aligned
## [`AngularVelocityA`].
AngularVelocity : {
    axis_of_rotation : UnitVector3.UnitVector3,
    angular_speed : Radians.Radians Binary32,
}

## Creates a new [`AngularVelocity`] with the given axis of rotation and
## angular speed.
new : UnitVector3.UnitVector3, Radians.Radians Binary32 -> AngularVelocity
new = |axis_of_rotation, angular_speed|
    { axis_of_rotation, angular_speed }

## Creates a new [`AngularVelocity`] from the given angular velocity
## vector.
from_vector : Vector3.Vector3 -> AngularVelocity
from_vector = |angular_velocity_vector|
    when UnitVector3.try_from_and_get(angular_velocity_vector, 1e-15) is
        Some((axis_of_rotation, angular_speed)) -> new(axis_of_rotation, angular_speed)
        None -> zero({})

## Creates a new [`AngularVelocity`] with zero angular speed.
zero : {} -> AngularVelocity
zero = |{}|
    { axis_of_rotation: UnitVector3.y_axis, angular_speed: 0.0 }

## Serializes a value of [AngularVelocity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AngularVelocity -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> UnitVector3.write_bytes(value.axis_of_rotation)
    |> Radians.write_bytes_32(value.angular_speed)

## Deserializes a value of [AngularVelocity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AngularVelocity _
from_bytes = |bytes|
    Ok(
        {
            axis_of_rotation: bytes |> List.sublist({ start: 0, len: 12 }) |> UnitVector3.from_bytes?,
            angular_speed: bytes |> List.sublist({ start: 12, len: 4 }) |> Radians.from_bytes_32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
