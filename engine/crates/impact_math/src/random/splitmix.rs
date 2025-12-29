//! Randomness using the `SplitMix` algorithm.

/// Generates a pseudo-random `u64` from the given `u64` value.
pub fn random_u64_from_state(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Generates a pseudo-random `u64` from the two given `u64` values.
pub fn random_u64_from_two_states(a: u64, b: u64) -> u64 {
    random_u64_from_state(a ^ random_u64_from_state(b))
}

/// Generates a pseudo-random `u64` from the three given `u64` values.
pub fn random_u64_from_three_states(a: u64, b: u64, c: u64) -> u64 {
    random_u64_from_two_states(random_u64_from_two_states(a, b), c)
}
