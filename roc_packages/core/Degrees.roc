module [
    Degrees,
    from_radians,
    write_bytes,
    from_bytes,
]

import Builtin

Degrees : F32

from_radians : F32 -> Degrees
from_radians = |radians|
    radians * 180 / Num.pi

write_bytes = Builtin.write_bytes_f32
from_bytes = Builtin.from_bytes_f32
