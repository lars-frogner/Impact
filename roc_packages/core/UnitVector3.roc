module [
    UnitVector3,
    UnitVector3F32,
    UnitVector3F64,
    is_approx_eq,
    map_to_f32,
    map_to_f64,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Vector3

UnitVector3 a : (Frac a, Frac a, Frac a)

UnitVector3F32 : UnitVector3 Binary32
UnitVector3F64 : UnitVector3 Binary64

map_to_f32 = Vector3.map_to_f32
map_to_f64 = Vector3.map_to_f64
is_approx_eq = Vector3.is_approx_eq
write_bytes_32 = Vector3.write_bytes_32
write_bytes_64 = Vector3.write_bytes_64
from_bytes_32 = Vector3.from_bytes_32
from_bytes_64 = Vector3.from_bytes_64
