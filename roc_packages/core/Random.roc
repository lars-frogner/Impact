module [
    Rng,
    new_rng,
    new_rng_stream,
    gen_u32,
    gen_u32_bounded,
    gen_u32_in_range,
    gen_f32,
    gen_f32_in_range,
]

Rng := Pcg32

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

## Generates a random U32 bekiw the given limit.
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
