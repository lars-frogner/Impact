module [
    Vector4,
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
    is_approx_eq,
    write_bytes,
    from_bytes,
]

import Builtin

Vector4 : (F32, F32, F32, F32)

zero = (0.0, 0.0, 0.0, 0.0)

same : F32 -> Vector4
same = |val|
    (val, val, val, val)

map : Vector4, (F32 -> F32) -> Vector4
map = |vec, f|
    (f(vec.0), f(vec.1), f(vec.2), f(vec.3))

map2 : Vector4, Vector4, (F32, F32 -> F32) -> Vector4
map2 = |a, b, f|
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2), f(a.3, b.3))

reduce : Vector4, (F32, F32 -> F32) -> F32
reduce = |vec, f|
    vec.0 |> f(vec.1) |> f(vec.2) |> f(vec.3)

add = |a, b| map2(a, b, Num.add)
sub = |a, b| map2(a, b, Num.sub)

scale = |vec, s| map(vec, |elem| Num.mul(elem, s))
unscale = |vec, s| scale(vec, 1.0 / s)

dot = |a, b| map2(a, b, Num.mul) |> reduce(Num.add)

norm_squared = |vec| dot(vec, vec)
norm = |vec| vec |> norm_squared |> Num.sqrt

is_approx_eq : Vector4, Vector4, { atol ?? F32, rtol ?? F32 } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)
    and Num.is_approx_eq(a.2, b.2, tol)
    and Num.is_approx_eq(a.3, b.3, tol)

write_bytes : List U8, Vector4 -> List U8
write_bytes = |bytes, (x, y, z, w)|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f32(x)
    |> Builtin.write_bytes_f32(y)
    |> Builtin.write_bytes_f32(z)
    |> Builtin.write_bytes_f32(w)

from_bytes : List U8 -> Result Vector4 Builtin.DecodeErr
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
        ),
    )
