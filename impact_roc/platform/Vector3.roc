module [
    Vector3,
    Vector3F32,
    Vector3F64,
    is_approx_eq,
    map_to_f32,
    map_to_f64,
]

Vector3 a : (Frac a, Frac a, Frac a)

Vector3F32 : Vector3 Binary32
Vector3F64 : Vector3 Binary64

map_to_f32 : Vector3 a -> Vector3F32
map_to_f32 = |vec|
    (Num.to_f32(vec.0), Num.to_f32(vec.1), Num.to_f32(vec.2))

map_to_f64 : Vector3 a -> Vector3F64
map_to_f64 = |vec|
    (Num.to_f64(vec.0), Num.to_f64(vec.1), Num.to_f64(vec.2))

is_approx_eq : Vector3 a, Vector3 a, { atol ?? Frac a, rtol ?? Frac a } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)
    and Num.is_approx_eq(a.2, b.2, tol)
