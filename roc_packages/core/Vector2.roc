module [
    Vector2,
    zeros,
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
    is_approx_eq,
    write_bytes,
    from_bytes,
]

import Builtin

Vector2 : (F32, F32)

zeros = (0.0, 0.0)

same : F32 -> Vector2
same = |val|
    (val, val)

map : Vector2, (F32 -> F32) -> Vector2
map = |vec, f|
    (f(vec.0), f(vec.1))

map2 : Vector2, Vector2, (F32, F32 -> F32) -> Vector2
map2 = |a, b, f|
    (f(a.0, b.0), f(a.1, b.1))

reduce : Vector2, (F32, F32 -> F32) -> F32
reduce = |vec, f|
    f(vec.0, vec.1)

add = |a, b| map2(a, b, Num.add)
sub = |a, b| map2(a, b, Num.sub)

scale = |vec, s| map(vec, |elem| Num.mul(elem, s))
unscale = |vec, s| scale(vec, 1.0 / s)

dot = |a, b| map2(a, b, Num.mul) |> reduce(Num.add)

norm_squared = |vec| dot(vec, vec)
norm = |vec| vec |> norm_squared |> Num.sqrt

is_approx_eq : Vector2, Vector2, { atol ?? F32, rtol ?? F32 } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)

write_bytes : List U8, Vector2 -> List U8
write_bytes = |bytes, (x, y)|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f32(x)
    |> Builtin.write_bytes_f32(y)

from_bytes : List U8 -> Result Vector2 Builtin.DecodeErr
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        ),
    )
