module [
    UnitVector3,
    UnitVector3F32,
    UnitVector3F64,
    x_axis,
    y_axis,
    z_axis,
    from,
    from_and_get,
    try_from_and_get,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Vector3 exposing [Vector3]

UnitVector3 a : (Frac a, Frac a, Frac a)

UnitVector3F32 : UnitVector3 Binary32
UnitVector3F64 : UnitVector3 Binary64

x_axis = (1.0, 0.0, 0.0)
y_axis = (0.0, 1.0, 0.0)
z_axis = (0.0, 0.0, 1.0)

from : Vector3 a -> UnitVector3 a
from = |vec|
    Vector3.unscale(vec, Vector3.norm(vec))

from_and_get : Vector3 a -> (UnitVector3 a, Frac a)
from_and_get = |vec|
    norm = Vector3.norm(vec)
    normalized = Vector3.unscale(vec, norm)
    (normalized, norm)

try_from_and_get : Vector3 a, Frac a -> [Some (UnitVector3 a, Frac a), None]
try_from_and_get = |vec, eps|
    norm = Vector3.norm(vec)
    if norm > eps then
        normalized = Vector3.unscale(vec, norm)
        Some((normalized, norm))
    else
        None

write_bytes_32 = Vector3.write_bytes_32
write_bytes_64 = Vector3.write_bytes_64
from_bytes_32 = Vector3.from_bytes_32
from_bytes_64 = Vector3.from_bytes_64
