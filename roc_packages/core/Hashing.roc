module [
    Hash32,
    Hash64,
    StringHash32,
    StringHash64,
    hash_str_32,
    hash_str_64,
    hash32,
    hash64,
    write_bytes_hash_32,
    write_bytes_hash_64,
    write_bytes_string_hash_32,
    write_bytes_string_hash_64,
    from_bytes_hash_32,
    from_bytes_hash_64,
    from_bytes_string_hash_32,
    from_bytes_string_hash_64,
]

import Builtin

Hash32 := U32 implements [Eq]
Hash64 := U64 implements [Eq]
StringHash32 : Hash32
StringHash64 : Hash64

## Hashes the given string into a [Hash32] using the FNV-a1 algorithm.
hash_str_32 : Str -> Hash32
hash_str_32 = |string|
    string |> Str.to_utf8 |> hash32

# Check that the result matches `compute_hash_str_32` in `impact_math`
expect hash_str_32("abcæøå!") == @Hash32(4050550160)

## Hashes the given string into a [Hash64] using the FNV-a1 algorithm.
hash_str_64 : Str -> Hash64
hash_str_64 = |string|
    string |> Str.to_utf8 |> hash64

# Check that the result matches `compute_hash_str_64` in `impact_math`
expect hash_str_64("abcæøå!") == @Hash64(14209695664012565680)

## Hashes the given list of bytes into a [Hash32] using the FNV-a1 algorithm.
hash32 : List U8 -> Hash32
hash32 = |bytes|
    fnv_offset_basis = 0x811c9dc5
    fnv_prime = 0x01000193
    @Hash32(
        bytes
        |> List.walk(
            fnv_offset_basis,
            |hash, byte|
                hash
                |> Num.bitwise_xor(Num.to_u32(byte))
                |> Num.mul_wrap(fnv_prime),
        ),
    )

## Hashes the given list of bytes into a [Hash64] using the FNV-a1 algorithm.
hash64 : List U8 -> Hash64
hash64 = |bytes|
    fnv_offset_basis = 0xcbf29ce484222325
    fnv_prime = 0x100000001b3
    @Hash64(
        bytes
        |> List.walk(
            fnv_offset_basis,
            |hash, byte|
                hash
                |> Num.bitwise_xor(Num.to_u64(byte))
                |> Num.mul_wrap(fnv_prime),
        ),
    )

write_bytes_hash_32 : List U8, Hash32 -> List U8
write_bytes_hash_32 = |bytes, @Hash32(hash)|
    Builtin.write_bytes_u32(bytes, hash)

write_bytes_hash_64 : List U8, Hash64 -> List U8
write_bytes_hash_64 = |bytes, @Hash64(hash)|
    Builtin.write_bytes_u64(bytes, hash)

write_bytes_string_hash_32 = write_bytes_hash_32
write_bytes_string_hash_64 = write_bytes_hash_64

from_bytes_hash_32 : List U8 -> Result Hash32 Builtin.DecodeErr
from_bytes_hash_32 = |bytes|
    Builtin.from_bytes_u32(bytes) |> Result.map_ok(@Hash32)

from_bytes_hash_64 : List U8 -> Result Hash64 Builtin.DecodeErr
from_bytes_hash_64 = |bytes|
    Builtin.from_bytes_u64(bytes) |> Result.map_ok(@Hash64)

from_bytes_string_hash_32 = from_bytes_hash_32
from_bytes_string_hash_64 = from_bytes_hash_64
