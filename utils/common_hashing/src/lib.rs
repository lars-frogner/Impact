//! A common place for hashing functions that multiple systems must agree upon.

/// Hashes the given bytes into a `u32` using the FNV-a1 algorithm.
#[inline]
pub const fn hash_bytes_to_u32(bytes: &[u8]) -> u32 {
    const FNV_OFFSET_BASIS: u32 = 0x811c9dc5;
    const FNV_PRIME: u32 = 0x01000193;

    let mut hash = FNV_OFFSET_BASIS;
    let mut idx = 0;

    while idx < bytes.len() {
        hash = (hash ^ bytes[idx] as u32).wrapping_mul(FNV_PRIME);
        idx += 1;
    }

    hash
}

/// Hashes the given bytes into a `u64` using the FNV-a1 algorithm.
#[inline]
pub const fn hash_bytes_to_u64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    let mut idx = 0;

    while idx < bytes.len() {
        hash = (hash ^ bytes[idx] as u64).wrapping_mul(FNV_PRIME);
        idx += 1;
    }

    hash
}

/// Hashes the given string into a `u32` using the FNV-a1 algorithm.
#[inline]
pub const fn hash_str_to_u32(string: &str) -> u32 {
    hash_bytes_to_u32(string.as_bytes())
}

/// Hashes the given string into a `u64` using the FNV-a1 algorithm.
#[inline]
pub const fn hash_str_to_u64(string: &str) -> u64 {
    hash_bytes_to_u64(string.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_str_to_u32_works() {
        assert_eq!(hash_str_to_u32("abcæøå!"), 4050550160);
    }

    #[test]
    fn hash_str_to_u64_works() {
        assert_eq!(hash_str_to_u64("abcæøå!"), 14209695664012565680);
    }
}
