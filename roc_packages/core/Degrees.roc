module [
    Degrees,
    from_radians,
    degrees,
    radians,
    write_bytes,
    from_bytes,
]

import Builtin

Degrees : F32

from_radians : F32 -> Degrees
from_radians = |rads|
    rads * 180 / Num.pi

degrees : Degrees -> F32
degrees = |degs|
    degs

radians : Degrees -> F32
radians = |degs|
    degs * Num.pi / 180

write_bytes = Builtin.write_bytes_f32
from_bytes = Builtin.from_bytes_f32
