module [
    Vector3,
    Vector3F32,
    Vector3F64,
    zero,
    same,
    map,
    map2,
    reduce,
    add,
    sub,
    scale,
    unscale,
    dot,
    norm_squared,
    norm,
    normalize,
    cross,
    is_approx_eq,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin

Vector3 a : (Frac a, Frac a, Frac a)

Vector3F32 : Vector3 Binary32
Vector3F64 : Vector3 Binary64

zero = (0.0, 0.0, 0.0)

same : Frac a -> Vector3 a
same = |val|
    (val, val, val)

map : Vector3 a, (Frac a -> Frac b) -> Vector3 b
map = |vec, f|
    (f(vec.0), f(vec.1), f(vec.2))

map2 : Vector3 a, Vector3 b, (Frac a, Frac b -> Frac c) -> Vector3 c
map2 = |a, b, f|
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))

reduce : Vector3 a, (Frac a, Frac a -> Frac a) -> Frac a
reduce = |vec, f|
    vec.0 |> f(vec.1) |> f(vec.2)

add = |a, b| map2(a, b, Num.add)
sub = |a, b| map2(a, b, Num.sub)

scale = |vec, s| map(vec, |elem| Num.mul(elem, s))
unscale = |vec, s| scale(vec, 1.0 / s)

dot = |a, b| map2(a, b, Num.mul) |> reduce(Num.add)

norm_squared = |vec| dot(vec, vec)
norm = |vec| vec |> norm_squared |> Num.sqrt

normalize = |vec| vec |> unscale(norm(vec))

cross = |(ax, ay, az), (bx, by, bz)|
    (
        ay * bz - az * by,
        az * bx - ax * bz,
        ax * by - ay * bx,
    )

is_approx_eq : Vector3 a, Vector3 a, { atol ?? Frac a, rtol ?? Frac a } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)
    and Num.is_approx_eq(a.2, b.2, tol)

write_bytes_32 : List U8, Vector3F32 -> List U8
write_bytes_32 = |bytes, (x, y, z)|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32(x)
    |> Builtin.write_bytes_f32(y)
    |> Builtin.write_bytes_f32(z)

write_bytes_64 : List U8, Vector3F64 -> List U8
write_bytes_64 = |bytes, (x, y, z)|
    bytes
    |> List.reserve(24)
    |> Builtin.write_bytes_f64(x)
    |> Builtin.write_bytes_f64(y)
    |> Builtin.write_bytes_f64(z)

from_bytes_32 : List U8 -> Result Vector3F32 Builtin.DecodeErr
from_bytes_32 = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
        ),
    )

from_bytes_64 : List U8 -> Result Vector3F64 Builtin.DecodeErr
from_bytes_64 = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
        ),
    )
