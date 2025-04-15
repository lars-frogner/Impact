module [
    UnitQuaternion,
    UnitQuaternionF32,
    UnitQuaternionF64,
    is_approx_eq,
    map_to_f32,
    map_to_f64,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Vector4

UnitQuaternion a : (Frac a, Frac a, Frac a, Frac a)

UnitQuaternionF32 : UnitQuaternion Binary32
UnitQuaternionF64 : UnitQuaternion Binary64

map_to_f32 = Vector4.map_to_f32
map_to_f64 = Vector4.map_to_f64
is_approx_eq = Vector4.is_approx_eq
write_bytes_32 = Vector4.write_bytes_32
write_bytes_64 = Vector4.write_bytes_64
from_bytes_32 = Vector4.from_bytes_32
from_bytes_64 = Vector4.from_bytes_64
