module [
    Matrix3,
    Matrix3F32,
    Matrix3F64,
    is_approx_eq,
    map_to_f32,
    map_to_f64,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin
import Vector3 exposing [Vector3]

Matrix3 a : (Vector3 a, Vector3 a, Vector3 a)

Matrix3F32 : Matrix3 Binary32
Matrix3F64 : Matrix3 Binary64

map_to_f32 : Matrix3 a -> Matrix3F32
map_to_f32 = |cols|
    (Vector3.map_to_f32(cols.0), Vector3.map_to_f32(cols.1), Vector3.map_to_f32(cols.2))

map_to_f64 : Matrix3 a -> Matrix3F64
map_to_f64 = |cols|
    (Vector3.map_to_f64(cols.0), Vector3.map_to_f64(cols.1), Vector3.map_to_f64(cols.2))

is_approx_eq : Matrix3 a, Matrix3 a, { atol ?? Frac a, rtol ?? Frac a } -> Bool
is_approx_eq = |a, b, tol|
    Vector3.is_approx_eq(a.0, b.0, tol)
    and Vector3.is_approx_eq(a.1, b.1, tol)
    and Vector3.is_approx_eq(a.2, b.2, tol)

write_bytes_32 : List U8, Matrix3F32 -> List U8
write_bytes_32 = |bytes, (col1, col2, col3)|
    bytes
    |> List.reserve(36)
    |> Vector3.write_bytes_32(col1)
    |> Vector3.write_bytes_32(col2)
    |> Vector3.write_bytes_32(col3)

write_bytes_64 : List U8, Matrix3F64 -> List U8
write_bytes_64 = |bytes, (col1, col2, col3)|
    bytes
    |> List.reserve(72)
    |> Vector3.write_bytes_64(col1)
    |> Vector3.write_bytes_64(col2)
    |> Vector3.write_bytes_64(col3)

from_bytes_32 : List U8 -> Result Matrix3F32 Builtin.DecodeErr
from_bytes_32 = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
            bytes |> List.sublist({ start: 12, len: 12 }) |> Vector3.from_bytes_32?,
            bytes |> List.sublist({ start: 24, len: 12 }) |> Vector3.from_bytes_32?,
        ),
    )

from_bytes_64 : List U8 -> Result Matrix3F64 Builtin.DecodeErr
from_bytes_64 = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            bytes |> List.sublist({ start: 24, len: 24 }) |> Vector3.from_bytes_64?,
            bytes |> List.sublist({ start: 48, len: 24 }) |> Vector3.from_bytes_64?,
        ),
    )
