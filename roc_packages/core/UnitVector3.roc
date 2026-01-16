module [
    UnitVector3,
    unit_x,
    unit_y,
    unit_z,
    neg_unit_x,
    neg_unit_y,
    neg_unit_z,
    from,
    from_and_get,
    try_from_and_get,
    write_bytes,
    from_bytes,
]

import Vector3 exposing [Vector3]

UnitVector3 : (F32, F32, F32)

unit_x = (1.0, 0.0, 0.0)
unit_y = (0.0, 1.0, 0.0)
unit_z = (0.0, 0.0, 1.0)
neg_unit_x = (-1.0, 0.0, 0.0)
neg_unit_y = (0.0, -1.0, 0.0)
neg_unit_z = (0.0, 0.0, -1.0)

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
