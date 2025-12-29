module [
    Radians,
    from_degrees,
    write_bytes,
    from_bytes,
]

import Builtin

Radians : F32

from_degrees : F32 -> Radians
from_degrees = |degrees|
    degrees * Num.pi / 180

write_bytes = Builtin.write_bytes_f32
from_bytes = Builtin.from_bytes_f32
