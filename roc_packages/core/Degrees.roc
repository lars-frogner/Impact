module [
    Degrees,
    DegreesF32,
    DegreesF64,
    from_radians,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Builtin

Degrees a : Frac a

DegreesF32 : Degrees Binary32
DegreesF64 : Degrees Binary64

from_radians : Frac a -> Degrees a
from_radians = |radians|
    radians * 180 / Num.pi

write_bytes_32 = Builtin.write_bytes_f32
write_bytes_64 = Builtin.write_bytes_f64
from_bytes_32 = Builtin.from_bytes_f32
from_bytes_64 = Builtin.from_bytes_f64
