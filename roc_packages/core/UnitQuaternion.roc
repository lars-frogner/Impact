module [
    UnitQuaternion,
    UnitQuaternionF32,
    UnitQuaternionF64,
    identity,
    from_axis_angle,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Radians exposing [Radians]
import UnitVector3 exposing [UnitVector3]
import Vector3 exposing [Vector3]
import Vector4

UnitQuaternion a : (Frac a, Frac a, Frac a, Frac a)

UnitQuaternionF32 : UnitQuaternion Binary32
UnitQuaternionF64 : UnitQuaternion Binary64

identity = from_parts(Vector3.zero, 1.0)

from_axis_angle : UnitVector3 a, Radians a -> UnitQuaternion a
from_axis_angle = |axis, angle|
    sin_half_angle = Num.sin(0.5 * angle)
    cos_half_angle = Num.cos(0.5 * angle)
    from_parts(Vector3.scale(axis, sin_half_angle), cos_half_angle)

from_parts : Vector3 a, Frac a -> UnitQuaternion a
from_parts = |vector, scalar|
    (vector.0, vector.1, vector.2, scalar)

write_bytes_32 = Vector4.write_bytes_32
write_bytes_64 = Vector4.write_bytes_64
from_bytes_32 = Vector4.from_bytes_32
from_bytes_64 = Vector4.from_bytes_64
