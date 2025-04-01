module [
    Vector4,
    Vector4F32,
    Vector4F64,
    is_approx_eq,
    map_to_f32,
    map_to_f64,
]

Vector4 a : (Frac a, Frac a, Frac a, Frac a)

Vector4F32 : Vector4 Binary32
Vector4F64 : Vector4 Binary64

map_to_f32 : Vector4 a -> Vector4F32
map_to_f32 = |vec|
    (Num.to_f32(vec.0), Num.to_f32(vec.1), Num.to_f32(vec.2), Num.to_f32(vec.3))

map_to_f64 : Vector4 a -> Vector4F64
map_to_f64 = |vec|
    (Num.to_f64(vec.0), Num.to_f64(vec.1), Num.to_f64(vec.2), Num.to_f64(vec.3))

is_approx_eq : Vector4 a, Vector4 a, { atol ?? Frac a, rtol ?? Frac a } -> Bool
is_approx_eq = |a, b, tol|
    Num.is_approx_eq(a.0, b.0, tol)
    and Num.is_approx_eq(a.1, b.1, tol)
    and Num.is_approx_eq(a.2, b.2, tol)
    and Num.is_approx_eq(a.3, b.3, tol)
