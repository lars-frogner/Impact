module [
    Capsule,
    new,
    write_bytes,
    from_bytes,
]

import Builtin
import Point3 exposing [Point3]
import Vector3 exposing [Vector3]

## A capsule represented by the starting point and displacement vector of the
## segment making up the central axis of the cylinder between the caps, as well
## as a radius.
Capsule : {
    segment_start : Point3,
    segment_vector : Vector3,
    radius : F32,
}

## Creates a new capsule with the given segment starting point, segment
## vector and radius.
new : Point3, Vector3, F32 -> Capsule
new = |segment_start, segment_vector, radius|
    # This can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    { segment_start, segment_vector, radius }

write_bytes : List U8, Capsule -> List U8
write_bytes = |bytes, { segment_start, segment_vector, radius }|
    bytes
    |> List.reserve(28)
    |> Point3.write_bytes(segment_start)
    |> Vector3.write_bytes(segment_vector)
    |> Builtin.write_bytes_f32(radius)

from_bytes : List U8 -> Result Capsule Builtin.DecodeErr
from_bytes = |bytes|
    Ok(
        {
            segment_start: bytes |> List.sublist({ start: 0, len: 12 }) |> Point3.from_bytes?,
            segment_vector: bytes |> List.sublist({ start: 12, len: 12 }) |> Vector3.from_bytes?,
            radius: bytes |> List.sublist({ start: 24, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )
