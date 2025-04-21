module [
    Vector2,
    Vector2F32,
    Vector2F64,
    map,
    map2,
    reduce,
    add,
    sub,
    scale,
    unscale,
    dot,
    is_approx_eq,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin

Vector2 a : (Frac a, Frac a)

Vector2F32 : Vector2 Binary32
Vector2F64 : Vector2 Binary64

map : Vector2 a, (Frac a -> Frac b) -> Vector2 b
map = |vec, f|
    (f(vec.0), f(vec.1))

map2 : Vector2 a, Vector2 b, (Frac a, Frac b -> Frac c) -> Vector2 c
map2 = |a, b, f|
    (f(a.0, b.0), f(a.1, b.1))

reduce : Vector2 a, (Frac a, Frac a -> Frac a) -> Frac a
reduce = |vec, f|
    f(vec.0, vec.1)

add = |a, b| map2(a, b, Num.add)
sub = |a, b| map2(a, b, Num.sub)

scale = |vec, s| map(vec, |elem| Num.mul(elem, s))
unscale = |vec, s| scale(vec, 1.0 / s)

dot = |a, b| map2(a, b, Num.mul) |> reduce(Num.add)

is_approx_eq : Vector2 a, Vector2 a, { atol ?? Frac a, rtol ?? Frac a } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)

write_bytes_32 : List U8, Vector2F32 -> List U8
write_bytes_32 = |bytes, (x, y)|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f32(x)
    |> Builtin.write_bytes_f32(y)

write_bytes_64 : List U8, Vector2F64 -> List U8
write_bytes_64 = |bytes, (x, y)|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f64(x)
    |> Builtin.write_bytes_f64(y)

from_bytes_32 : List U8 -> Result Vector2F32 Builtin.DecodeErr
from_bytes_32 = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        ),
    )

from_bytes_64 : List U8 -> Result Vector2F64 Builtin.DecodeErr
from_bytes_64 = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
        ),
    )
