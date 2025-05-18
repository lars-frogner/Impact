module [
    DecodeErr,
    write_bytes_u8,
    write_bytes_u16,
    write_bytes_u32,
    write_bytes_u64,
    write_bytes_u128,
    write_bytes_i8,
    write_bytes_i16,
    write_bytes_i32,
    write_bytes_i64,
    write_bytes_i128,
    write_bytes_f32,
    write_bytes_f64,
    from_bytes_u8,
    from_bytes_u16,
    from_bytes_u32,
    from_bytes_u64,
    from_bytes_u128,
    from_bytes_i8,
    from_bytes_i16,
    from_bytes_i32,
    from_bytes_i64,
    from_bytes_i128,
    from_bytes_f32,
    from_bytes_f64,
]

# Encoding

## Assumes little-endian byte ordering
write_bytes_uint : List U8, Int *, U64 -> List U8
write_bytes_uint = |bytes, value, n_bytes|
    # Enabling this expect breaks FFI
    # expect List.contains([2, 4, 8, 16], n_bytes)

    List.range({ start: At 0, end: Length n_bytes })
    |> List.walk(
        List.reserve(bytes, n_bytes),
        |bts, idx|
            bts
            |> List.append(
                value
                |> Num.shift_right_zf_by(Num.to_u8(idx * 8))
                |> Num.to_u8,
            ),
    )

# Unsigned integers

write_bytes_u8 : List U8, U8 -> List U8
write_bytes_u8 = |bytes, value|
    bytes |> List.append(value)

write_bytes_u16 : List U8, U16 -> List U8
write_bytes_u16 = |bytes, value|
    bytes |> write_bytes_uint(value, 2)

write_bytes_u32 : List U8, U32 -> List U8
write_bytes_u32 = |bytes, value|
    bytes |> write_bytes_uint(value, 4)

write_bytes_u64 : List U8, U64 -> List U8
write_bytes_u64 = |bytes, value|
    bytes |> write_bytes_uint(value, 8)

write_bytes_u128 : List U8, U128 -> List U8
write_bytes_u128 = |bytes, value|
    bytes |> write_bytes_uint(value, 16)

# Signed integers

write_bytes_i8 : List U8, I8 -> List U8
write_bytes_i8 = |bytes, value|
    bytes |> write_bytes_u8(Num.to_u8(value))

write_bytes_i16 : List U8, I16 -> List U8
write_bytes_i16 = |bytes, value|
    bytes |> write_bytes_u16(Num.to_u16(value))

write_bytes_i32 : List U8, I32 -> List U8
write_bytes_i32 = |bytes, value|
    bytes |> write_bytes_u32(Num.to_u32(value))

write_bytes_i64 : List U8, I64 -> List U8
write_bytes_i64 = |bytes, value|
    bytes |> write_bytes_u64(Num.to_u64(value))

write_bytes_i128 : List U8, I128 -> List U8
write_bytes_i128 = |bytes, value|
    bytes |> write_bytes_u128(Num.to_u128(value))

# IEEE 754 floating-point numbers

write_bytes_f32 : List U8, F32 -> List U8
write_bytes_f32 = |bytes, value|
    bytes |> write_bytes_u32(Num.f32_to_bits(value))

write_bytes_f64 : List U8, F64 -> List U8
write_bytes_f64 = |bytes, value|
    bytes |> write_bytes_u64(Num.f64_to_bits(value))

# Decoding

DecodeErr : [
    InvalidNumberOfBytes,
    MissingDiscriminant,
    InvalidDiscriminant U8,
]

## Assumes little-endian byte ordering
from_bytes_uint : List U8, (U8 -> Int a) -> Int a
from_bytes_uint = |bytes, cast_byte_to_target|
    bytes
    |> List.map_with_index(
        |byte, idx| byte |> cast_byte_to_target |> Num.shift_left_by(Num.to_u8(idx * 8)),
    )
    |> List.walk(0, Num.bitwise_or)

# Unsigned integers

from_bytes_u8 : List U8 -> Result U8 DecodeErr
from_bytes_u8 = |bytes|
    if List.len(bytes) == 1 then
        Ok(from_bytes_uint(bytes, Num.to_u8))
    else
        Err(InvalidNumberOfBytes)

expect [] |> write_bytes_u8(0x12) |> from_bytes_u8 == Ok(0x12)

from_bytes_u16 : List U8 -> Result U16 DecodeErr
from_bytes_u16 = |bytes|
    if List.len(bytes) == 2 then
        Ok(from_bytes_uint(bytes, Num.to_u16))
    else
        Err(InvalidNumberOfBytes)

expect [] |> write_bytes_u16(0x1234) |> from_bytes_u16 == Ok(0x1234)

from_bytes_u32 : List U8 -> Result U32 DecodeErr
from_bytes_u32 = |bytes|
    if List.len(bytes) == 4 then
        Ok(from_bytes_uint(bytes, Num.to_u32))
    else
        Err(InvalidNumberOfBytes)

expect [] |> write_bytes_u32(0x12345678) |> from_bytes_u32 == Ok(0x12345678)

from_bytes_u64 : List U8 -> Result U64 DecodeErr
from_bytes_u64 = |bytes|
    if List.len(bytes) == 8 then
        Ok(from_bytes_uint(bytes, Num.to_u64))
    else
        Err(InvalidNumberOfBytes)

expect [] |> write_bytes_u64(0x123456789abcdef9) |> from_bytes_u64 == Ok(0x123456789abcdef9)

from_bytes_u128 : List U8 -> Result U128 DecodeErr
from_bytes_u128 = |bytes|
    if List.len(bytes) == 16 then
        Ok(from_bytes_uint(bytes, Num.to_u128))
    else
        Err(InvalidNumberOfBytes)

expect
    []
    |> write_bytes_u128(0x123456789abcdef987654321fedcba89)
    |> from_bytes_u128
    == Ok(0x123456789abcdef987654321fedcba89)

# Signed integers

from_bytes_i8 : List U8 -> Result I8 DecodeErr
from_bytes_i8 = |bytes|
    bytes |> from_bytes_u8 |> Result.map_ok(Num.to_i8)

expect [] |> write_bytes_i8(0x12) |> from_bytes_i8 == Ok(0x12)
expect [] |> write_bytes_i8(-0x12) |> from_bytes_i8 == Ok(-0x12)

from_bytes_i16 : List U8 -> Result I16 DecodeErr
from_bytes_i16 = |bytes|
    bytes |> from_bytes_u16 |> Result.map_ok(Num.to_i16)

expect [] |> write_bytes_i16(0x1234) |> from_bytes_i16 == Ok(0x1234)
expect [] |> write_bytes_i16(-0x1234) |> from_bytes_i16 == Ok(-0x1234)

from_bytes_i32 : List U8 -> Result I32 DecodeErr
from_bytes_i32 = |bytes|
    bytes |> from_bytes_u32 |> Result.map_ok(Num.to_i32)

expect [] |> write_bytes_i32(0x12345678) |> from_bytes_i32 == Ok(0x12345678)
expect [] |> write_bytes_i32(-0x12345678) |> from_bytes_i32 == Ok(-0x12345678)

from_bytes_i64 : List U8 -> Result I64 DecodeErr
from_bytes_i64 = |bytes|
    bytes |> from_bytes_u64 |> Result.map_ok(Num.to_i64)

expect [] |> write_bytes_i64(0x123456789abcdef9) |> from_bytes_i64 == Ok(0x123456789abcdef9)
expect [] |> write_bytes_i64(-0x123456789abcdef9) |> from_bytes_i64 == Ok(-0x123456789abcdef9)

from_bytes_i128 : List U8 -> Result I128 DecodeErr
from_bytes_i128 = |bytes|
    bytes |> from_bytes_u128 |> Result.map_ok(Num.to_i128)

expect
    []
    |> write_bytes_i128(0x123456789abcdef987654321fedcba89)
    |> from_bytes_i128
    == Ok(0x123456789abcdef987654321fedcba89)
expect
    []
    |> write_bytes_i128(-0x123456789abcdef987654321fedcba89)
    |> from_bytes_i128
    == Ok(-0x123456789abcdef987654321fedcba89)

# IEEE 754 floating-point numbers

from_bytes_f32 : List U8 -> Result F32 DecodeErr
from_bytes_f32 = |bytes|
    bits = from_bytes_u32(bytes)?
    Ok(Num.f32_from_bits(bits))

expect
    input = 3.14
    output = [] |> write_bytes_f32(input) |> from_bytes_f32
    when output is
        Ok(out) if Num.is_approx_eq(out, input, {}) -> Bool.true
        _ -> Bool.false

from_bytes_f64 : List U8 -> Result F64 DecodeErr
from_bytes_f64 = |bytes|
    bits = from_bytes_u64(bytes)?
    Ok(Num.f64_from_bits(bits))

expect
    input = 3.14
    output = [] |> write_bytes_f64(input) |> from_bytes_f64
    when output is
        Ok(out) if Num.is_approx_eq(out, input, {}) -> Bool.true
        _ -> Bool.false
