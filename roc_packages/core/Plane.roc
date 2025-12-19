module [
    Plane,
    new,
    write_bytes,
    from_bytes,
]

import Builtin
import UnitVector3 exposing [UnitVector3F32]

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
Plane : {
    unit_normal : UnitVector3F32,
    displacement : F32,
}

## Creates a new plane defined by the given unit normal
## vector and displacement.
new : UnitVector3F32, F32 -> Plane
new = |unit_normal, displacement|
    { unit_normal, displacement }

write_bytes : List U8, Plane -> List U8
write_bytes = |bytes, { unit_normal, displacement }|
    bytes
    |> List.reserve(16)
    |> UnitVector3.write_bytes_32(unit_normal)
    |> Builtin.write_bytes_f32(displacement)

from_bytes : List U8 -> Result Plane Builtin.DecodeErr
from_bytes = |bytes|
    Ok(
        {
            unit_normal: bytes |> List.sublist({ start: 0, len: 12 }) |> UnitVector3.from_bytes_32?,
            displacement: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )
