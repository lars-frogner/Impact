module [
    clamp,
]

clamp : Num a, Num a, Num a -> Num a
clamp = |value, lower, upper|
    Num.max(lower, Num.min(upper, value))
