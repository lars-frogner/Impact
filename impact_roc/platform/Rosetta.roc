module [
    DecodeErr,
    RustRoundtripErr,
    u16_write_bytes,
    u32_write_bytes,
    u64_write_bytes,
    u128_write_bytes,
    i16_write_bytes,
    i32_write_bytes,
    i64_write_bytes,
    i128_write_bytes,
    f32_write_bytes!,
    f64_write_bytes!,
    vec3_f32_write_bytes!,
    vec4_f32_write_bytes!,
    vec3_f64_write_bytes!,
    vec4_f64_write_bytes!,
    u16_from_bytes,
    u32_from_bytes,
    u64_from_bytes,
    u128_from_bytes,
    i16_from_bytes,
    i32_from_bytes,
    i64_from_bytes,
    i128_from_bytes,
    f32_from_bytes!,
    f64_from_bytes!,
    vec3_f32_from_bytes!,
    vec4_f32_from_bytes!,
    vec3_f64_from_bytes!,
    vec4_f64_from_bytes!,
    vec3_f32_rust_roundtrip!,
    vec4_f32_rust_roundtrip!,
    vec3_f64_rust_roundtrip!,
    vec4_f64_rust_roundtrip!,
    RoundtripTestStruct,
    test_struct_rust_roundtrip!,
    roundtrip_test_struct_eq,
]

import Host
import Vector3 exposing [Vector3F32, Vector3F64]
import Vector4 exposing [Vector4F32, Vector4F64]

# Encoding

## Assumes little-endian byte ordering
uint_write_bytes : List U8, Int *, U64 -> List U8
uint_write_bytes = |bytes, value, n_bytes|
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

u16_write_bytes : List U8, U16 -> List U8
u16_write_bytes = |bytes, value|
    bytes |> uint_write_bytes(value, 2)

u32_write_bytes : List U8, U32 -> List U8
u32_write_bytes = |bytes, value|
    bytes |> uint_write_bytes(value, 4)

u64_write_bytes : List U8, U64 -> List U8
u64_write_bytes = |bytes, value|
    bytes |> uint_write_bytes(value, 8)

u128_write_bytes : List U8, U128 -> List U8
u128_write_bytes = |bytes, value|
    bytes |> uint_write_bytes(value, 16)

# Signed integers

i16_write_bytes : List U8, I16 -> List U8
i16_write_bytes = |bytes, value|
    bytes |> u16_write_bytes(Num.to_u16(value))

i32_write_bytes : List U8, I32 -> List U8
i32_write_bytes = |bytes, value|
    bytes |> u32_write_bytes(Num.to_u32(value))

i64_write_bytes : List U8, I64 -> List U8
i64_write_bytes = |bytes, value|
    bytes |> u64_write_bytes(Num.to_u64(value))

i128_write_bytes : List U8, I128 -> List U8
i128_write_bytes = |bytes, value|
    bytes |> u128_write_bytes(Num.to_u128(value))

# IEEE 754 floating-point numbers (need to use FFI for now)

f32_write_bytes! : List U8, F32 => List U8
f32_write_bytes! = |bytes, value|
    bytes |> u32_write_bytes(Host.f32_to_bits!(value))

f64_write_bytes! : List U8, F64 => List U8
f64_write_bytes! = |bytes, value|
    bytes |> u64_write_bytes(Host.f64_to_bits!(value))

# Fixed-size floating-point vectors

vec3_f32_write_bytes! : List U8, Vector3F32 => List U8
vec3_f32_write_bytes! = |bytes, (x, y, z)|
    bytes
    |> List.reserve(12)
    |> f32_write_bytes!(x)
    |> f32_write_bytes!(y)
    |> f32_write_bytes!(z)

vec4_f32_write_bytes! : List U8, Vector4F32 => List U8
vec4_f32_write_bytes! = |bytes, (x, y, z, w)|
    bytes
    |> List.reserve(16)
    |> f32_write_bytes!(x)
    |> f32_write_bytes!(y)
    |> f32_write_bytes!(z)
    |> f32_write_bytes!(w)

vec3_f64_write_bytes! : List U8, Vector3F64 => List U8
vec3_f64_write_bytes! = |bytes, (x, y, z)|
    bytes
    |> List.reserve(24)
    |> f64_write_bytes!(x)
    |> f64_write_bytes!(y)
    |> f64_write_bytes!(z)

vec4_f64_write_bytes! : List U8, Vector4F64 => List U8
vec4_f64_write_bytes! = |bytes, (x, y, z, w)|
    bytes
    |> List.reserve(32)
    |> f64_write_bytes!(x)
    |> f64_write_bytes!(y)
    |> f64_write_bytes!(z)
    |> f64_write_bytes!(w)

# Decoding

DecodeErr : [
    InvalidNumberOfBytes,
]

## Assumes little-endian byte ordering
uint_from_bytes : List U8, (U8 -> Int a) -> Int a
uint_from_bytes = |bytes, cast_byte_to_target|
    bytes
    |> List.map_with_index(
        |byte, idx| byte |> cast_byte_to_target |> Num.shift_left_by(Num.to_u8(idx * 8)),
    )
    |> List.walk(0, Num.bitwise_or)

# Unsigned integers

u16_from_bytes : List U8 -> Result U16 DecodeErr
u16_from_bytes = |bytes|
    if List.len(bytes) == 2 then
        Ok(uint_from_bytes(bytes, Num.to_u16))
    else
        Err(InvalidNumberOfBytes)

expect [] |> u16_write_bytes(0x1234) |> u16_from_bytes == Ok(0x1234)

u32_from_bytes : List U8 -> Result U32 DecodeErr
u32_from_bytes = |bytes|
    if List.len(bytes) == 4 then
        Ok(uint_from_bytes(bytes, Num.to_u32))
    else
        Err(InvalidNumberOfBytes)

expect [] |> u32_write_bytes(0x12345678) |> u32_from_bytes == Ok(0x12345678)

u64_from_bytes : List U8 -> Result U64 DecodeErr
u64_from_bytes = |bytes|
    if List.len(bytes) == 8 then
        Ok(uint_from_bytes(bytes, Num.to_u64))
    else
        Err(InvalidNumberOfBytes)

expect [] |> u64_write_bytes(0x123456789abcdef9) |> u64_from_bytes == Ok(0x123456789abcdef9)

u128_from_bytes : List U8 -> Result U128 DecodeErr
u128_from_bytes = |bytes|
    if List.len(bytes) == 16 then
        Ok(uint_from_bytes(bytes, Num.to_u128))
    else
        Err(InvalidNumberOfBytes)

expect
    []
    |> u128_write_bytes(0x123456789abcdef987654321fedcba89)
    |> u128_from_bytes
    == Ok(0x123456789abcdef987654321fedcba89)

# Signed integers

i16_from_bytes : List U8 -> Result I16 DecodeErr
i16_from_bytes = |bytes|
    bytes |> u16_from_bytes |> Result.map_ok(Num.to_i16)

expect [] |> i16_write_bytes(0x1234) |> i16_from_bytes == Ok(0x1234)
expect [] |> i16_write_bytes(-0x1234) |> i16_from_bytes == Ok(-0x1234)

i32_from_bytes : List U8 -> Result I32 DecodeErr
i32_from_bytes = |bytes|
    bytes |> u32_from_bytes |> Result.map_ok(Num.to_i32)

expect [] |> i32_write_bytes(0x12345678) |> i32_from_bytes == Ok(0x12345678)
expect [] |> i32_write_bytes(-0x12345678) |> i32_from_bytes == Ok(-0x12345678)

i64_from_bytes : List U8 -> Result I64 DecodeErr
i64_from_bytes = |bytes|
    bytes |> u64_from_bytes |> Result.map_ok(Num.to_i64)

expect [] |> i64_write_bytes(0x123456789abcdef9) |> i64_from_bytes == Ok(0x123456789abcdef9)
expect [] |> i64_write_bytes(-0x123456789abcdef9) |> i64_from_bytes == Ok(-0x123456789abcdef9)

i128_from_bytes : List U8 -> Result I128 DecodeErr
i128_from_bytes = |bytes|
    bytes |> u128_from_bytes |> Result.map_ok(Num.to_i128)

expect
    []
    |> i128_write_bytes(0x123456789abcdef987654321fedcba89)
    |> i128_from_bytes
    == Ok(0x123456789abcdef987654321fedcba89)
expect
    []
    |> i128_write_bytes(-0x123456789abcdef987654321fedcba89)
    |> i128_from_bytes
    == Ok(-0x123456789abcdef987654321fedcba89)

# IEEE 754 floating-point numbers (need to use FFI for now)

f32_from_bytes! : List U8 => Result F32 DecodeErr
f32_from_bytes! = |bytes|
    bits = u32_from_bytes(bytes)?
    Ok(Host.f32_from_bits!(bits))

f64_from_bytes! : List U8 => Result F64 DecodeErr
f64_from_bytes! = |bytes|
    bits = u64_from_bytes(bytes)?
    Ok(Host.f64_from_bits!(bits))

# Fixed-size floating-point vectors

vec3_f32_from_bytes! : List U8 => Result Vector3F32 DecodeErr
vec3_f32_from_bytes! = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> f32_from_bytes!?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> f32_from_bytes!?,
            bytes |> List.sublist({ start: 8, len: 4 }) |> f32_from_bytes!?,
        ),
    )

vec4_f32_from_bytes! : List U8 => Result Vector4F32 DecodeErr
vec4_f32_from_bytes! = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> f32_from_bytes!?,
            bytes |> List.sublist({ start: 4, len: 4 }) |> f32_from_bytes!?,
            bytes |> List.sublist({ start: 8, len: 4 }) |> f32_from_bytes!?,
            bytes |> List.sublist({ start: 12, len: 4 }) |> f32_from_bytes!?,
        ),
    )

vec3_f64_from_bytes! : List U8 => Result Vector3F64 DecodeErr
vec3_f64_from_bytes! = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> f64_from_bytes!?,
            bytes |> List.sublist({ start: 8, len: 8 }) |> f64_from_bytes!?,
            bytes |> List.sublist({ start: 16, len: 8 }) |> f64_from_bytes!?,
        ),
    )

vec4_f64_from_bytes! : List U8 => Result Vector4F64 DecodeErr
vec4_f64_from_bytes! = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> f64_from_bytes!?,
            bytes |> List.sublist({ start: 8, len: 8 }) |> f64_from_bytes!?,
            bytes |> List.sublist({ start: 16, len: 8 }) |> f64_from_bytes!?,
            bytes |> List.sublist({ start: 24, len: 8 }) |> f64_from_bytes!?,
        ),
    )

# Roc -> Rust -> Roc roundtrip testing

RustRoundtripErr : [
    FromRust Str,
    Decode DecodeErr,
]

vec3_f32_rust_roundtrip! : Vector3F32 => Result Vector3F32 RustRoundtripErr
vec3_f32_rust_roundtrip! = |vec|
    []
    |> vec3_f32_write_bytes!(vec)
    |> Host.vec3_f32_roundtrip!
    |> Result.map_err(FromRust)?
    |> vec3_f32_from_bytes!
    |> Result.map_err(Decode)

vec4_f32_rust_roundtrip! : Vector4F32 => Result Vector4F32 RustRoundtripErr
vec4_f32_rust_roundtrip! = |vec|
    []
    |> vec4_f32_write_bytes!(vec)
    |> Host.vec4_f32_roundtrip!
    |> Result.map_err(FromRust)?
    |> vec4_f32_from_bytes!
    |> Result.map_err(Decode)

vec3_f64_rust_roundtrip! : Vector3F64 => Result Vector3F64 RustRoundtripErr
vec3_f64_rust_roundtrip! = |vec|
    []
    |> vec3_f64_write_bytes!(vec)
    |> Host.vec3_f64_roundtrip!
    |> Result.map_err(FromRust)?
    |> vec3_f64_from_bytes!
    |> Result.map_err(Decode)

vec4_f64_rust_roundtrip! : Vector4F64 => Result Vector4F64 RustRoundtripErr
vec4_f64_rust_roundtrip! = |vec|
    []
    |> vec4_f64_write_bytes!(vec)
    |> Host.vec4_f64_roundtrip!
    |> Result.map_err(FromRust)?
    |> vec4_f64_from_bytes!
    |> Result.map_err(Decode)

RoundtripTestStruct : {
    field_1 : Vector3F32,
    field_2 : F32,
    field_3 : Vector4F64,
    field_4 : F64,
    field_5 : Vector3F64,
    field_6 : Vector4F32,
    field_7 : U64,
    field_8 : U32,
    field_9 : I32,
    field_10 : I64,
}

roundtrip_test_struct_write_bytes! : List U8, RoundtripTestStruct => List U8
roundtrip_test_struct_write_bytes! = |bytes, struct|
    bytes
    |> vec3_f32_write_bytes!(struct.field_1)
    |> f32_write_bytes!(struct.field_2)
    |> vec4_f64_write_bytes!(struct.field_3)
    |> f64_write_bytes!(struct.field_4)
    |> vec3_f64_write_bytes!(struct.field_5)
    |> vec4_f32_write_bytes!(struct.field_6)
    |> u64_write_bytes(struct.field_7)
    |> u32_write_bytes(struct.field_8)
    |> i32_write_bytes(struct.field_9)
    |> i64_write_bytes(struct.field_10)

roundtrip_test_struct_from_bytes! : List U8 => Result RoundtripTestStruct DecodeErr
roundtrip_test_struct_from_bytes! = |bytes|
    Ok(
        {
            field_1: bytes |> List.sublist({ start: 0, len: 12 }) |> vec3_f32_from_bytes!?,
            field_2: bytes |> List.sublist({ start: 12, len: 4 }) |> f32_from_bytes!?,
            field_3: bytes |> List.sublist({ start: 16, len: 32 }) |> vec4_f64_from_bytes!?,
            field_4: bytes |> List.sublist({ start: 48, len: 8 }) |> f64_from_bytes!?,
            field_5: bytes |> List.sublist({ start: 56, len: 24 }) |> vec3_f64_from_bytes!?,
            field_6: bytes |> List.sublist({ start: 80, len: 16 }) |> vec4_f32_from_bytes!?,
            field_7: bytes |> List.sublist({ start: 96, len: 8 }) |> u64_from_bytes?,
            field_8: bytes |> List.sublist({ start: 104, len: 4 }) |> u32_from_bytes?,
            field_9: bytes |> List.sublist({ start: 108, len: 4 }) |> i32_from_bytes?,
            field_10: bytes |> List.sublist({ start: 112, len: 8 }) |> i64_from_bytes?,
        },
    )

test_struct_rust_roundtrip! : RoundtripTestStruct => Result RoundtripTestStruct RustRoundtripErr
test_struct_rust_roundtrip! = |struct|
    []
    |> roundtrip_test_struct_write_bytes!(struct)
    |> Host.test_struct_roundtrip!
    |> Result.map_err(FromRust)?
    |> roundtrip_test_struct_from_bytes!
    |> Result.map_err(Decode)

roundtrip_test_struct_eq : RoundtripTestStruct, RoundtripTestStruct -> Bool
roundtrip_test_struct_eq = |a, b|
    Vector3.is_approx_eq(a.field_1, b.field_1, {})
    and
    Num.is_approx_eq(a.field_2, b.field_2, {})
    and
    Vector4.is_approx_eq(a.field_3, b.field_3, {})
    and
    Num.is_approx_eq(a.field_4, b.field_4, {})
    and
    Vector3.is_approx_eq(a.field_5, b.field_5, {})
    and
    Vector4.is_approx_eq(a.field_6, b.field_6, {})
    and
    a.field_7
    == b.field_7
    and
    a.field_8
    == b.field_8
    and
    a.field_9
    == b.field_9
    and
    a.field_10
    == b.field_10
