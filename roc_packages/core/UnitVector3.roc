module [
    UnitVector3,
    x_axis,
    y_axis,
    z_axis,
    neg_x_axis,
    neg_y_axis,
    neg_z_axis,
    from,
    from_and_get,
    try_from_and_get,
    write_bytes,
    from_bytes,
]

import Vector3 exposing [Vector3]

UnitVector3 : (F32, F32, F32)

x_axis = (1.0, 0.0, 0.0)
y_axis = (0.0, 1.0, 0.0)
z_axis = (0.0, 0.0, 1.0)
neg_x_axis = (-1.0, 0.0, 0.0)
neg_y_axis = (0.0, -1.0, 0.0)
neg_z_axis = (0.0, 0.0, -1.0)

from : Vector3 -> UnitVector3
from = |vec|
    Vector3.unscale(vec, Vector3.norm(vec))

from_and_get : Vector3 -> (UnitVector3, F32)
from_and_get = |vec|
    norm = Vector3.norm(vec)
    normalized = Vector3.unscale(vec, norm)
    (normalized, norm)

try_from_and_get : Vector3, F32 -> [Some (UnitVector3, F32), None]
try_from_and_get = |vec, eps|
    norm = Vector3.norm(vec)
    if norm > eps then
        normalized = Vector3.unscale(vec, norm)
        Some((normalized, norm))
    else
        None

write_bytes = Vector3.write_bytes
from_bytes = Vector3.from_bytes
