module [
    Radians,
    from_degrees,
    radians,
    degrees,
    write_bytes,
    from_bytes,
]

import Builtin

Radians : F32

from_degrees : F32 -> Radians
from_degrees = |degs|
    degs * Num.pi / 180

radians : Radians -> F32
radians = |rads|
    rads

degrees : Radians -> F32
degrees = |rads|
    rads * 180 / Num.pi

write_bytes = Builtin.write_bytes_f32
from_bytes = Builtin.from_bytes_f32
