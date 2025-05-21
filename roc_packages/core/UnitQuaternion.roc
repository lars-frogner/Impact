module [
    UnitQuaternion,
    UnitQuaternionF32,
    UnitQuaternionF64,
    identity,
    from_axis_angle,
    from_parts,
    parts,
    mul,
    norm_squared,
    norm,
    normalize,
    rotate_vector,
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

parts : UnitQuaternion a -> (Vector3 a, Frac a)
parts = |(x, y, z, w)|
    ((x, y, z), w)

mul : UnitQuaternion a, UnitQuaternion a -> UnitQuaternion a
mul = |a, b|
    (a_imag, a_real) = parts(a)
    (b_imag, b_real) = parts(b)

    real = a_real * b_real - Vector3.dot(a_imag, b_imag)

    imag =
        Vector3.cross(a_imag, b_imag)
        |> Vector3.add(Vector3.scale(b_imag, a_real))
        |> Vector3.add(Vector3.scale(a_imag, b_real))

    from_parts(imag, real)

norm_squared : UnitQuaternion a -> Frac a
norm_squared = |quat|
    (imag, real) = parts(quat)
    Vector3.norm_squared(imag) + real * real

norm : UnitQuaternion a -> Frac a
norm = |quat|
    Num.sqrt(norm_squared(quat))

normalize : UnitQuaternion a -> UnitQuaternion a
normalize = |quat|
    inv_norm = 1.0 / norm(quat)
    (imag, real) = parts(quat)
    from_parts(Vector3.scale(imag, inv_norm), real * inv_norm)

rotate_vector : UnitQuaternion a, Vector3 a -> Vector3 a
rotate_vector = |quat, vec|
    (imag, real) = parts(quat)
    tmp = Vector3.cross(imag, vec) |> Vector3.scale(2.0)
    vec
    |> Vector3.add(Vector3.scale(tmp, real))
    |> Vector3.add(Vector3.cross(imag, tmp))

write_bytes_32 = Vector4.write_bytes_32
write_bytes_64 = Vector4.write_bytes_64
from_bytes_32 = Vector4.from_bytes_32
from_bytes_64 = Vector4.from_bytes_64
