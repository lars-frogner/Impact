module [
    Radians,
    RadiansF32,
    RadiansF64,
    to_degrees,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin

Radians a : Frac a

RadiansF32 : Radians Binary32
RadiansF64 : Radians Binary64

to_degrees : Radians a -> Frac a
to_degrees = |radians|
    radians * 180 / Num.pi

write_bytes_32 = Builtin.write_bytes_f32
write_bytes_64 = Builtin.write_bytes_f64
from_bytes_32 = Builtin.from_bytes_f32
from_bytes_64 = Builtin.from_bytes_f64
