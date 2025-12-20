module [
    Matrix3,
    map,
    map2,
    add,
    sub,
    scale,
    unscale,
    is_approx_eq,
    write_bytes,
    from_bytes,
]

import Builtin
import Vector3 exposing [Vector3F32]

Matrix3 : (Vector3F32, Vector3F32, Vector3F32)

map : Matrix3, (F32 -> F32) -> Matrix3
map = |mat, f|
    (Vector3.map(mat.0, f), Vector3.map(mat.1, f), Vector3.map(mat.2, f))

map2 : Matrix3, Matrix3, (F32, F32 -> F32) -> Matrix3
map2 = |a, b, f|
    (Vector3.map2(a.0, b.0, f), Vector3.map2(a.1, b.1, f), Vector3.map2(a.2, b.2, f))

add = |a, b| map2(a, b, Num.add)
sub = |a, b| map2(a, b, Num.sub)

scale = |mat, s| map(mat, |elem| Num.mul(elem, s))
unscale = |mat, s| scale(mat, 1.0 / s)

is_approx_eq : Matrix3, Matrix3, { atol ?? F32, rtol ?? F32 } -> Bool
is_approx_eq = |a, b, tol|
    Vector3.is_approx_eq(a.0, b.0, tol)
    and Vector3.is_approx_eq(a.1, b.1, tol)
    and Vector3.is_approx_eq(a.2, b.2, tol)

write_bytes : List U8, Matrix3 -> List U8
write_bytes = |bytes, (col1, col2, col3)|
    bytes
    |> List.reserve(36)
    |> Vector3.write_bytes_32(col1)
    |> Vector3.write_bytes_32(col2)
    |> Vector3.write_bytes_32(col3)

from_bytes : List U8 -> Result Matrix3 Builtin.DecodeErr
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
            bytes |> List.sublist({ start: 12, len: 12 }) |> Vector3.from_bytes_32?,
            bytes |> List.sublist({ start: 24, len: 12 }) |> Vector3.from_bytes_32?,
        ),
    )
