module [
    Plane,
    PlaneF32,
    PlaneF64,
    new,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin
import UnitVector3 exposing [UnitVector3]

## A plane in 3D, represented by a unit normal and
## a displacement.
##
## The displacement `d` can be determined from the
## normal `n` and any point `p` lying on the plane
## as `d = -n.dot(p)`. By storing the displacement
## instead of the point, we remove redundate degrees
## of freedom.
##
## The plane divides space into two halfspaces, the
## positive and negative halfspace. The positive one
## is defined as the halfspace the unit normal is
## pointing into.
Plane a : {
    unit_normal : UnitVector3 a,
    displacement : Frac a,
}

PlaneF32 : Plane Binary32
PlaneF64 : Plane Binary64

## Creates a new plane defined by the given unit normal
## vector and displacement.
new : UnitVector3 a, Frac a -> Plane a
new = |unit_normal, displacement|
    { unit_normal, displacement }

write_bytes_32 : List U8, PlaneF32 -> List U8
write_bytes_32 = |bytes, { unit_normal, displacement }|
    bytes
    |> List.reserve(16)
    |> UnitVector3.write_bytes_32(unit_normal)
    |> Builtin.write_bytes_f32(displacement)

write_bytes_64 : List U8, PlaneF64 -> List U8
write_bytes_64 = |bytes, { unit_normal, displacement }|
    bytes
    |> List.reserve(32)
    |> UnitVector3.write_bytes_64(unit_normal)
    |> Builtin.write_bytes_f64(displacement)

from_bytes_32 : List U8 -> Result PlaneF32 Builtin.DecodeErr
from_bytes_32 = |bytes|
    Ok(
        {
            unit_normal: bytes |> List.sublist({ start: 0, len: 12 }) |> UnitVector3.from_bytes_32?,
            displacement: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

from_bytes_64 : List U8 -> Result PlaneF64 Builtin.DecodeErr
from_bytes_64 = |bytes|
    Ok(
        {
            unit_normal: bytes |> List.sublist({ start: 0, len: 24 }) |> UnitVector3.from_bytes_64?,
            displacement: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )
