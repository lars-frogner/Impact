app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout exposing [IOErr]
import pf.Rosetta exposing [DecodeErr, RustRoundtripErr, RoundtripTestStruct]
import pf.Vector3 exposing [Vector3, Vector3F32, Vector3F64]
import pf.Vector4 exposing [Vector4, Vector4F32, Vector4F64]

TestErr : [
    Decode DecodeErr,
    RustRoundtrip RustRoundtripErr,
    NotEqualF32 F64 F64,
    NotEqualVec3 Vector3F64 Vector3F64,
    NotEqualVec4 Vector4F64 Vector4F64,
    NotEqualTestStruct RoundtripTestStruct RoundtripTestStruct,
    Std [StdoutErr IOErr],
    A {
            hey : U64,
        },
    C,
]

main! : {} => Result {} TestErr
main! = |{}|
    test_f32_roundtrip!(3.14)?
    test_f32_roundtrip!(-3.14)?
    test_f64_roundtrip!(3.14)?
    test_f64_roundtrip!(-3.14)?

    test_vector3_f32_roundtrip!((1.0, 2.0, 3.0))?
    test_vector3_f64_roundtrip!((1.0, 2.0, 3.0))?
    test_vector4_f32_roundtrip!((1.0, 2.0, 3.0, 4.0))?
    test_vector4_f64_roundtrip!((1.0, 2.0, 3.0, 4.0))?

    test_vector3_f32_rust_roundtrip!((1.0, 2.0, 3.0))?
    test_vector4_f32_rust_roundtrip!((1.0, 2.0, 3.0, 4.0))?
    test_vector3_f64_rust_roundtrip!((1.0, 2.0, 3.0))?
    test_vector4_f64_rust_roundtrip!((1.0, 2.0, 3.0, 4.0))?

    test_struct_rust_roundtrip!(
        {
            field_1: (1.0, 2.0, 3.0),
            field_2: 4.0,
            field_3: (5.0, 6.0, 7.0, 8.0),
            field_4: 9.0,
            field_5: (10.0, 11.0, 12.0),
            field_6: (13.0, 14.0, 15.0, 16.0),
            field_7: 17,
            field_8: 18,
            field_9: -19,
            field_10: -20,
        },
    )?

    Stdout.line!("All tests passed") |> Result.map_err(Std)

test_f32_roundtrip! : F32 => Result {} TestErr
test_f32_roundtrip! = |value|
    test_float_roundtrip!(value, Rosetta.f32_write_bytes!, Rosetta.f32_from_bytes!)

test_f64_roundtrip! : F64 => Result {} TestErr
test_f64_roundtrip! = |value|
    test_float_roundtrip!(value, Rosetta.f64_write_bytes!, Rosetta.f64_from_bytes!)

test_vector3_f32_roundtrip! : Vector3F32 => Result {} TestErr
test_vector3_f32_roundtrip! = |vec|
    test_vector3_roundtrip!(vec, Rosetta.vector3_f32_write_bytes!, Rosetta.vector3_f32_from_bytes!)

test_vector4_f32_roundtrip! : Vector4F32 => Result {} TestErr
test_vector4_f32_roundtrip! = |vec|
    test_vector4_roundtrip!(vec, Rosetta.vector4_f32_write_bytes!, Rosetta.vector4_f32_from_bytes!)

test_vector3_f64_roundtrip! : Vector3F64 => Result {} TestErr
test_vector3_f64_roundtrip! = |vec|
    test_vector3_roundtrip!(vec, Rosetta.vector3_f64_write_bytes!, Rosetta.vector3_f64_from_bytes!)

test_vector4_f64_roundtrip! : Vector4F64 => Result {} TestErr
test_vector4_f64_roundtrip! = |vec|
    test_vector4_roundtrip!(vec, Rosetta.vector4_f64_write_bytes!, Rosetta.vector4_f64_from_bytes!)

test_vector3_f32_rust_roundtrip! : Vector3F32 => Result {} TestErr
test_vector3_f32_rust_roundtrip! = |vec|
    test_vector3_rust_roundtrip!(vec, Rosetta.vector3_f32_rust_roundtrip!)

test_vector4_f32_rust_roundtrip! : Vector4F32 => Result {} TestErr
test_vector4_f32_rust_roundtrip! = |vec|
    test_vector4_rust_roundtrip!(vec, Rosetta.vector4_f32_rust_roundtrip!)

test_vector3_f64_rust_roundtrip! : Vector3F64 => Result {} TestErr
test_vector3_f64_rust_roundtrip! = |vec|
    test_vector3_rust_roundtrip!(vec, Rosetta.vector3_f64_rust_roundtrip!)

test_vector4_f64_rust_roundtrip! : Vector4F64 => Result {} TestErr
test_vector4_f64_rust_roundtrip! = |vec|
    test_vector4_rust_roundtrip!(vec, Rosetta.vector4_f64_rust_roundtrip!)

test_float_roundtrip! : Frac a, (List U8, Frac a => List U8), (List U8 => Result (Frac a) DecodeErr) => Result {} TestErr
test_float_roundtrip! = |value, encode!, decode!|
    res = [] |> encode!(value) |> decode! |> Result.map_err(Decode)?
    if Num.is_approx_eq(res, value, {}) then
        Ok({})
    else
        Err(NotEqualF32(Num.to_f64(res), Num.to_f64(value)))

test_vector3_roundtrip! : Vector3 a, (List U8, Vector3 a => List U8), (List U8 => Result (Vector3 a) DecodeErr) => Result {} TestErr
test_vector3_roundtrip! = |vec, encode!, decode!|
    res = [] |> encode!(vec) |> decode! |> Result.map_err(Decode)?
    if Vector3.is_approx_eq(res, vec, {}) then
        Ok({})
    else
        Err(NotEqualVec3(Vector3.map_to_f64(res), Vector3.map_to_f64(vec)))

test_vector4_roundtrip! : Vector4 a, (List U8, Vector4 a => List U8), (List U8 => Result (Vector4 a) DecodeErr) => Result {} TestErr
test_vector4_roundtrip! = |vec, encode!, decode!|
    res = [] |> encode!(vec) |> decode! |> Result.map_err(Decode)?
    if Vector4.is_approx_eq(res, vec, {}) then
        Ok({})
    else
        Err(NotEqualVec4(Vector4.map_to_f64(res), Vector4.map_to_f64(vec)))

test_vector3_rust_roundtrip! : Vector3 a, (Vector3 a => Result (Vector3 a) RustRoundtripErr) => Result {} TestErr
test_vector3_rust_roundtrip! = |vec, roundtrip!|
    res = vec |> roundtrip! |> Result.map_err(RustRoundtrip)?
    if Vector3.is_approx_eq(res, vec, {}) then
        Ok({})
    else
        Err(NotEqualVec3(Vector3.map_to_f64(res), Vector3.map_to_f64(vec)))

test_vector4_rust_roundtrip! : Vector4 a, (Vector4 a => Result (Vector4 a) RustRoundtripErr) => Result {} TestErr
test_vector4_rust_roundtrip! = |vec, roundtrip!|
    res = vec |> roundtrip! |> Result.map_err(RustRoundtrip)?
    if Vector4.is_approx_eq(res, vec, {}) then
        Ok({})
    else
        Err(NotEqualVec4(Vector4.map_to_f64(res), Vector4.map_to_f64(vec)))

test_struct_rust_roundtrip! : RoundtripTestStruct => Result {} TestErr
test_struct_rust_roundtrip! = |struct|
    res = struct |> Rosetta.test_struct_rust_roundtrip! |> Result.map_err(RustRoundtrip)?
    if Rosetta.roundtrip_test_struct_eq(res, struct) then
        Ok({})
    else
        Err(NotEqualTestStruct(res, struct))
