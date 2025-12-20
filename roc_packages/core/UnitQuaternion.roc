module [
    UnitQuaternion,
    identity,
    from_axis_angle,
    from_parts,
    parts,
    mul,
    norm_squared,
    norm,
    normalize,
    invert,
    rotate_vector,
    to_rotation_matrix,
    write_bytes,
    from_bytes,
]

import Radians exposing [RadiansF32]
import UnitVector3 exposing [UnitVector3F32]
import Vector3 exposing [Vector3F32]
import Vector4
import Matrix3 exposing [Matrix3]

UnitQuaternion : (F32, F32, F32, F32)

identity = from_parts(Vector3.zero, 1.0)

from_axis_angle : UnitVector3F32, RadiansF32 -> UnitQuaternion
from_axis_angle = |axis, angle|
    sin_half_angle = Num.sin(0.5 * angle)
    cos_half_angle = Num.cos(0.5 * angle)
    from_parts(Vector3.scale(axis, sin_half_angle), cos_half_angle)

from_parts : Vector3F32, F32 -> UnitQuaternion
from_parts = |vector, scalar|
    (vector.0, vector.1, vector.2, scalar)

parts : UnitQuaternion -> (Vector3F32, F32)
parts = |(x, y, z, w)|
    ((x, y, z), w)

mul : UnitQuaternion, UnitQuaternion -> UnitQuaternion
mul = |a, b|
    (a_imag, a_real) = parts(a)
    (b_imag, b_real) = parts(b)

    real = a_real * b_real - Vector3.dot(a_imag, b_imag)

    imag =
        Vector3.cross(a_imag, b_imag)
        |> Vector3.add(Vector3.scale(b_imag, a_real))
        |> Vector3.add(Vector3.scale(a_imag, b_real))

    from_parts(imag, real)

norm_squared : UnitQuaternion -> F32
norm_squared = |quat|
    (imag, real) = parts(quat)
    Vector3.norm_squared(imag) + real * real

norm : UnitQuaternion -> F32
norm = |quat|
    Num.sqrt(norm_squared(quat))

normalize : UnitQuaternion -> UnitQuaternion
normalize = |quat|
    inv_norm = 1.0 / norm(quat)
    (imag, real) = parts(quat)
    from_parts(Vector3.scale(imag, inv_norm), real * inv_norm)

invert : UnitQuaternion -> UnitQuaternion
invert = |quat|
    (imag, real) = parts(quat)
    from_parts(Vector3.flip(imag), real)

rotate_vector : UnitQuaternion, Vector3F32 -> Vector3F32
rotate_vector = |quat, vec|
    (imag, real) = parts(quat)
    tmp = Vector3.cross(imag, vec) |> Vector3.scale(2.0)
    vec
    |> Vector3.add(Vector3.scale(tmp, real))
    |> Vector3.add(Vector3.cross(imag, tmp))

to_rotation_matrix : UnitQuaternion -> Matrix3
to_rotation_matrix = |(x, y, z, w)|
    x2 = 2 * x * x
    y2 = 2 * y * y
    z2 = 2 * z * z
    xy = 2 * x * y
    xz = 2 * x * z
    yz = 2 * y * z
    wx = 2 * w * x
    wy = 2 * w * y
    wz = 2 * w * z
    col1 = (1 - (y2 + z2), (xy + wz), (xz - wy))
    col2 = ((xy - wz), 1 - (x2 + z2), (yz + wx))
    col3 = ((xz + wy), (yz - wx), 1 - (x2 + y2))
    (col1, col2, col3)

write_bytes = Vector4.write_bytes_32
from_bytes = Vector4.from_bytes_32
