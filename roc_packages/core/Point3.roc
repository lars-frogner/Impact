module [
    Point3,
    Point3F32,
    Point3F64,
    origin,
    map,
    map2,
    is_approx_eq,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Vector3

Point3 a : (Frac a, Frac a, Frac a)

Point3F32 : Point3 Binary32
Point3F64 : Point3 Binary64

origin = (0.0, 0.0, 0.0)

map = Vector3.map
map2 = Vector3.map2
is_approx_eq = Vector3.is_approx_eq
write_bytes_32 = Vector3.write_bytes_32
write_bytes_64 = Vector3.write_bytes_64
from_bytes_32 = Vector3.from_bytes_32
from_bytes_64 = Vector3.from_bytes_64
