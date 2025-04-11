module [
    Vector3,
    Vector3F32,
    Vector3F64,
    is_approx_eq,
    map_to_f32,
    map_to_f64,
    write_bytes_32!,
    write_bytes_64!,
    from_bytes_32!,
    from_bytes_64!,
]

import Core.Builtin as Builtin

Vector3 a : (Frac a, Frac a, Frac a)

Vector3F32 : Vector3 Binary32
Vector3F64 : Vector3 Binary64

map_to_f32 : Vector3 a -> Vector3F32
map_to_f32 = |vec|
    (Num.to_f32(vec.0), Num.to_f32(vec.1), Num.to_f32(vec.2))

map_to_f64 : Vector3 a -> Vector3F64
map_to_f64 = |vec|
    (Num.to_f64(vec.0), Num.to_f64(vec.1), Num.to_f64(vec.2))

is_approx_eq : Vector3 a, Vector3 a, { atol ?? Frac a, rtol ?? Frac a } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)
    and Num.is_approx_eq(a.2, b.2, tol)

write_bytes_32! : List U8, Vector3F32 => List U8
write_bytes_32! = |bytes, (x, y, z)|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32!(x)
    |> Builtin.write_bytes_f32!(y)
    |> Builtin.write_bytes_f32!(z)

write_bytes_64! : List U8, Vector3F64 => List U8
write_bytes_64! = |bytes, (x, y, z)|
    bytes
    |> List.reserve(24)
    |> Builtin.write_bytes_f64!(x)
    |> Builtin.write_bytes_f64!(y)
    |> Builtin.write_bytes_f64!(z)

from_bytes_32! : List U8 => Result Vector3F32 Builtin.DecodeErr
from_bytes_32! = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32!?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32!?,
            bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32!?,
        ),
    )

from_bytes_64! : List U8 => Result Vector3F64 Builtin.DecodeErr
from_bytes_64! = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64!?,
            bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64!?,
            bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64!?,
        ),
    )
