module [
    Sphere,
    SphereF32,
    SphereF64,
    new,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin
import Point3 exposing [Point3]

## A sphere represented by the center point and the radius.
Sphere a : {
    center : Point3 a,
    radius : Frac a,
}

SphereF32 : Sphere Binary32
SphereF64 : Sphere Binary64

## Creates a new sphere with the given center and radius.
##
## # Panics
## If `radius` is negative.
new : Point3 a, Frac a -> Sphere a
new = |center, radius|
    # This can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    { center, radius }

write_bytes_32 : List U8, SphereF32 -> List U8
write_bytes_32 = |bytes, { center, radius }|
    bytes
    |> List.reserve(16)
    |> Point3.write_bytes_32(center)
    |> Builtin.write_bytes_f32(radius)

write_bytes_64 : List U8, SphereF64 -> List U8
write_bytes_64 = |bytes, { center, radius }|
    bytes
    |> List.reserve(32)
    |> Point3.write_bytes_64(center)
    |> Builtin.write_bytes_f64(radius)

from_bytes_32 : List U8 -> Result SphereF32 Builtin.DecodeErr
from_bytes_32 = |bytes|
    Ok(
        {
            center: bytes |> List.sublist({ start: 0, len: 12 }) |> Point3.from_bytes_32?,
            radius: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

from_bytes_64 : List U8 -> Result SphereF64 Builtin.DecodeErr
from_bytes_64 = |bytes|
    Ok(
        {
            center: bytes |> List.sublist({ start: 0, len: 24 }) |> Point3.from_bytes_64?,
            radius: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )
