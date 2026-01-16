module [
    Point3,
    origin,
    map,
    map2,
    translate,
    distance_between,
    squared_distance_between,
    is_approx_eq,
    write_bytes,
    from_bytes,
]

import Vector3 exposing [Vector3]

Point3 : (F32, F32, F32)

origin = (0.0, 0.0, 0.0)

translate : Point3, Vector3 -> Point3
translate = |point, translation|
    Vector3.add(point, translation)

squared_distance_between : Point3, Point3 -> F32
squared_distance_between = |point_a, point_b|
    Vector3.norm_squared(Vector3.sub(point_b, point_a))

distance_between : Point3, Point3 -> F32
distance_between = |point_a, point_b|
    Vector3.norm(Vector3.sub(point_b, point_a))

map = Vector3.map
map2 = Vector3.map2
is_approx_eq = Vector3.is_approx_eq
write_bytes = Vector3.write_bytes
from_bytes = Vector3.from_bytes
