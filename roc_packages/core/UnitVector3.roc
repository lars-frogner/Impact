module [
    UnitVector3,
    UnitVector3F32,
    UnitVector3F64,
    x_axis,
    y_axis,
    z_axis,
    write_bytes_32,
    write_bytes_64,
    from_bytes_32,
    from_bytes_64,
]

import Vector3

UnitVector3 a : (Frac a, Frac a, Frac a)

UnitVector3F32 : UnitVector3 Binary32
UnitVector3F64 : UnitVector3 Binary64

x_axis = (1.0, 0.0, 0.0)
y_axis = (0.0, 1.0, 0.0)
z_axis = (0.0, 0.0, 1.0)

write_bytes_32 = Vector3.write_bytes_32
write_bytes_64 = Vector3.write_bytes_64
from_bytes_32 = Vector3.from_bytes_32
from_bytes_64 = Vector3.from_bytes_64
