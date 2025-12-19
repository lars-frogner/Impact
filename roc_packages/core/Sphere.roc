module [
    Sphere,
    new,
    write_bytes,
    from_bytes,
]

import Builtin
import Point3 exposing [Point3F32]

## A sphere represented by the center point and the radius.
Sphere : {
    center : Point3F32,
    radius : F32,
}

## Creates a new sphere with the given center and radius.
##
## # Panics
## If `radius` is negative.
new : Point3F32, F32 -> Sphere
new = |center, radius|
    # This can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    { center, radius }

write_bytes : List U8, Sphere -> List U8
write_bytes = |bytes, { center, radius }|
    bytes
    |> List.reserve(16)
    |> Point3.write_bytes_32(center)
    |> Builtin.write_bytes_f32(radius)

from_bytes : List U8 -> Result Sphere Builtin.DecodeErr
from_bytes = |bytes|
    Ok(
        {
            center: bytes |> List.sublist({ start: 0, len: 12 }) |> Point3.from_bytes_32?,
            radius: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )
