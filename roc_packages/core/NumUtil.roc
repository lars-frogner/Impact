module [
    clamp,
    modulo,
]

clamp : Num a, Num a, Num a -> Num a
clamp = |value, lower, upper|
    Num.max(lower, Num.min(upper, value))

modulo : Frac a, Frac a -> Frac a
modulo = |a, b|
    divided = a / b
    floored = Num.floor(divided)
    a - b * Num.to_frac(floored)
