module [
    Vector3,
    zeros,
    same,
    map,
    map2,
    reduce,
    add,
    sub,
    scale,
    unscale,
    flipped,
    dot,
    norm_squared,
    norm,
    normalized,
    cross,
    is_approx_eq,
    write_bytes,
    from_bytes,
]

import Builtin

Vector3 : (F32, F32, F32)

zeros = (0.0, 0.0, 0.0)

same : F32 -> Vector3
same = |val|
    (val, val, val)

map : Vector3, (F32 -> F32) -> Vector3
map = |vec, f|
    (f(vec.0), f(vec.1), f(vec.2))

map2 : Vector3, Vector3, (F32, F32 -> F32) -> Vector3
map2 = |a, b, f|
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))

reduce : Vector3, (F32, F32 -> F32) -> F32
reduce = |vec, f|
    vec.0 |> f(vec.1) |> f(vec.2)

add = |a, b| map2(a, b, Num.add)
sub = |a, b| map2(a, b, Num.sub)

scale = |vec, s| map(vec, |elem| Num.mul(elem, s))
unscale = |vec, s| scale(vec, 1.0 / s)

flipped = |vec| (-vec.0, -vec.1, -vec.2)

dot = |a, b| map2(a, b, Num.mul) |> reduce(Num.add)

norm_squared = |vec| dot(vec, vec)
norm = |vec| vec |> norm_squared |> Num.sqrt

normalized = |vec| vec |> unscale(norm(vec))

cross = |(ax, ay, az), (bx, by, bz)|
    (
        ay * bz - az * by,
        az * bx - ax * bz,
        ax * by - ay * bx,
    )

is_approx_eq : Vector3, Vector3, { atol ?? F32, rtol ?? F32 } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)
    and Num.is_approx_eq(a.2, b.2, tol)

write_bytes : List U8, Vector3 -> List U8
write_bytes = |bytes, (x, y, z)|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32(x)
    |> Builtin.write_bytes_f32(y)
    |> Builtin.write_bytes_f32(z)

from_bytes : List U8 -> Result Vector3 Builtin.DecodeErr
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
        ),
    )
