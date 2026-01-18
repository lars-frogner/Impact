module [
    Rng,
    Gaussian,
    PowerLaw,
    new_rng,
    new_rng_stream,
    gen_u32,
    gen_u32_bounded,
    gen_u32_in_range,
    gen_f32,
    gen_f32_in_range,
    gen_two_f32_normal,
    gen_f32_normal,
    gen_two_f32_gaussian,
    gen_f32_gaussian,
    gen_f32_power_law,
]

Rng := Pcg32

Gaussian : {
    mean : F32,
    std_dev : F32,
}

PowerLaw : {
    exponent : F32,
    min_value : F32,
    max_value : F32,
}

new_rng : U64 -> Rng
new_rng = |seed|
    incr = 1442695040888963407_u64
    @Rng(new_pcg32(seed, incr))

new_rng_stream : U64, U64 -> Rng
new_rng_stream = |seed, stream|
    incr = stream |> Num.mul_wrap(2) |> Num.add_wrap(1) # Ensure odd
    @Rng(new_pcg32(seed, incr))

## Generates a random U32.
gen_u32 : Rng -> (Rng, U32)
gen_u32 = |@Rng(rng)|
    (next_rng, out) = gen_pcg32(rng)
    (@Rng(next_rng), out)

## Generates a random U32 below the given limit.
gen_u32_bounded : Rng, U32 -> (Rng, U32)
gen_u32_bounded = |rng, limit|
    # Reject generated values below the size of the biased zone
    threshold = Num.sub_wrap(0, limit) % limit

    try_gen = |r|
        (next_r, out) = gen_u32(r)
        if out >= threshold then
            (next_r, out % limit)
        else
            try_gen(next_r)

    try_gen(rng)

## Generates a random U32 in the given range (inclusive).
gen_u32_in_range : Rng, U32, U32 -> (Rng, U32)
gen_u32_in_range = |rng, min, max|
    limit = max |> Num.sub_saturated(min) |> Num.add_saturated(1)
    (next_rng, out) = gen_u32_bounded(rng, limit)
    (next_rng, min |> Num.add_saturated(out))

## Generates a random F32 in the range [0.0, 1.0).
gen_f32 : Rng -> (Rng, F32)
gen_f32 = |rng|
    (next_rng, random_u32) = gen_u32(rng)
    out =
        random_u32
        # Keep 24 bits (mantissa precision)
        |> Num.shift_right_zf_by(8)
        # Cast to f32
        |> Num.to_f32
        # Divide by 2^24 to normalize
        |> Num.mul(5.9604645e-8)
    (next_rng, out)

## Generates a random F32 in the given range (exclusive).
gen_f32_in_range : Rng, F32, F32 -> (Rng, F32)
gen_f32_in_range = |rng, min, max|
    (next_rng, random_frac) = gen_f32(rng)
    (next_rng, min + random_frac * (max - min))

## Generates two random F32 values sampled from the standard normal distribution
## using the Marsaglia polar method.
gen_two_f32_normal : Rng -> (Rng, F32, F32)
gen_two_f32_normal = |rng|
    try_gen = |r|
        (r2, x) = gen_f32_in_range(r, -1.0, 1.0)
        (r3, y) = gen_f32_in_range(r2, -1.0, 1.0)
        s = x * x + y * y
        if s < 1.0 then
            scale = Num.sqrt(-2 * Num.log(s) / s)
            (r3, x * scale, y * scale)
        else
            try_gen(r3)

    try_gen(rng)

## Generates a random F32 value sampled from the standard normal distribution
## using the Marsaglia polar method.
gen_f32_normal : Rng -> (Rng, F32)
gen_f32_normal = |rng|
    (next_rng, x, _ignored) = gen_two_f32_normal(rng)
    (next_rng, x)

## Generates two random F32 values sampled from the given Gaussian distribution
## using the Marsaglia polar method.
gen_two_f32_gaussian : Rng, Gaussian -> (Rng, F32, F32)
gen_two_f32_gaussian = |rng, { mean, std_dev }|
    (next_rng, x, y) = gen_two_f32_normal(rng)
    (next_rng, mean + std_dev * x, mean + std_dev * y)

## Generates a random F32 value sampled from the given Gaussian distribution,
## using the Marsaglia polar method.
gen_f32_gaussian : Rng, Gaussian -> (Rng, F32)
gen_f32_gaussian = |rng, { mean, std_dev }|
    (next_rng, x) = gen_f32_normal(rng)
    (next_rng, mean + std_dev * x)

## Generates a random F32 value sampled from the given power-law distribution.
gen_f32_power_law : Rng, PowerLaw -> (Rng, F32)
gen_f32_power_law = |rng, power_law|
    (next_rng, prob) = gen_f32(rng)
    out = eval_inverse_cumulative_power_law(power_law, prob)
    (next_rng, out)

eval_inverse_cumulative_power_law : PowerLaw, F32 -> F32
eval_inverse_cumulative_power_law = |{ exponent, min_value, max_value }, cumul_prob|
    exponent_p1 = exponent + 1
    if Num.abs(exponent_p1) > 1e-3 then
        min_value_pow = Num.pow(min_value, exponent_p1)
        max_value_pow = Num.pow(max_value, exponent_p1)
        Num.pow((max_value_pow - min_value_pow) * cumul_prob + min_value_pow, 1 / exponent_p1)
    else
        min_value * Num.pow(max_value / min_value, cumul_prob) # exponent = -1

# PCG-XSH-RR

Pcg32 := {
    state : U64,
    incr : U64,
}

new_pcg32 : U64, U64 -> Pcg32
new_pcg32 = |seed, incr|
    state =
        0
        |> advance_state_pcg32(incr)
        |> Num.add_wrap(seed)
        |> advance_state_pcg32(incr)
    @Pcg32 { state, incr }

gen_pcg32 : Pcg32 -> (Pcg32, U32)
gen_pcg32 = |@Pcg32 { state, incr }|
    new_state = advance_state_pcg32(state, incr)
    count = state |> Num.shift_right_zf_by(59) |> Num.to_u32
    out =
        state
        |> Num.bitwise_xor(state |> Num.shift_right_zf_by(18))
        |> Num.shift_right_zf_by(27)
        |> Num.to_u32
        |> rotr32(count)
    (@Pcg32 { state: new_state, incr }, out)

advance_state_pcg32 = |state, incr|
    multiplier = 6364136223846793005_u64
    state |> Num.mul_wrap(multiplier) |> Num.add_wrap(incr)

rotr32 : U32, U32 -> U32
rotr32 = |val, count|
    val
    |> Num.shift_right_zf_by(Num.to_u8(count))
    |> Num.bitwise_or(
        val
        |> Num.shift_left_by(
            Num.sub_wrap(0, count)
            |> Num.bitwise_and(31)
            |> Num.to_u8,
        ),
    )
