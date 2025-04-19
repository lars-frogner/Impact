module [
    Degrees,
    DegreesF32,
    DegreesF64,
    Radians,
    RadiansF32,
    RadiansF64,
    to_radians,
    to_degrees,
    write_bytes_degrees_32,
    write_bytes_degrees_64,
    write_bytes_radians_32,
    write_bytes_radians_64,
    from_bytes_degrees_32,
    from_bytes_degrees_64,
    from_bytes_radians_32,
    from_bytes_radians_64,
]

import Builtin

Degrees a := Frac a

DegreesF32 : Degrees Binary32
DegreesF64 : Degrees Binary64

Radians a := Frac a

RadiansF32 : Radians Binary32
RadiansF64 : Radians Binary64

to_radians : Degrees a -> Radians a
to_radians = |@Degrees(degrees)|
    @Radians(degrees * Num.pi / 180)

to_degrees : Radians a -> Degrees a
to_degrees = |@Radians(radians)|
    @Degrees(radians * 180 / Num.pi)

write_bytes_degrees_32 : List U8, DegreesF32 -> List U8
write_bytes_degrees_32 = |bytes, @Degrees(degrees)|
    Builtin.write_bytes_f32(bytes, degrees)

write_bytes_degrees_64 : List U8, DegreesF64 -> List U8
write_bytes_degrees_64 = |bytes, @Degrees(degrees)|
    Builtin.write_bytes_f64(bytes, degrees)

write_bytes_radians_32 : List U8, RadiansF32 -> List U8
write_bytes_radians_32 = |bytes, @Radians(radians)|
    Builtin.write_bytes_f32(bytes, radians)

write_bytes_radians_64 : List U8, RadiansF64 -> List U8
write_bytes_radians_64 = |bytes, @Radians(radians)|
    Builtin.write_bytes_f64(bytes, radians)

from_bytes_degrees_32 : List U8 -> Result DegreesF32 Builtin.DecodeErr
from_bytes_degrees_32 = |bytes|
    Builtin.from_bytes_f32(bytes) |> Result.map_ok(@Degrees)

from_bytes_degrees_64 : List U8 -> Result DegreesF64 Builtin.DecodeErr
from_bytes_degrees_64 = |bytes|
    Builtin.from_bytes_f64(bytes) |> Result.map_ok(@Degrees)

from_bytes_radians_32 : List U8 -> Result RadiansF32 Builtin.DecodeErr
from_bytes_radians_32 = |bytes|
    Builtin.from_bytes_f32(bytes) |> Result.map_ok(@Radians)

from_bytes_radians_64 : List U8 -> Result RadiansF64 Builtin.DecodeErr
from_bytes_radians_64 = |bytes|
    Builtin.from_bytes_f64(bytes) |> Result.map_ok(@Radians)
