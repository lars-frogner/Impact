module [
    Radians,
    RadiansF32,
    RadiansF64,
    from_degrees,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin

Radians a : Frac a

RadiansF32 : Radians Binary32
RadiansF64 : Radians Binary64

from_degrees : Frac a -> Radians a
from_degrees = |degrees|
    degrees * Num.pi / 180

write_bytes_32 = Builtin.write_bytes_f32
write_bytes_64 = Builtin.write_bytes_f64
from_bytes_32 = Builtin.from_bytes_f32
from_bytes_64 = Builtin.from_bytes_f64
