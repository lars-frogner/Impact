# Hash: 030a927f23a03a8d8439499a3260e642df96100644956abeda3e194a8144f2ae
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::control::motion::MotionDirection
# Type category: Inline
# Commit: d505d37
module [
    MotionDirection,
    write_bytes,
    from_bytes,
]


## Possible directions of motion in the local coordinate
## system.
MotionDirection : [
    Forwards,
    Backwards,
    Right,
    Left,
    Up,
    Down,
]

## Serializes a value of [MotionDirection] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MotionDirection -> List U8
write_bytes = |bytes, value|
    when value is
        Forwards ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Backwards ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Right ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        Left ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        Up ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        Down ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

## Deserializes a value of [MotionDirection] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MotionDirection _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Forwards)
            [1, ..] -> Ok(Backwards)
            [2, ..] -> Ok(Right)
            [3, ..] -> Ok(Left)
            [4, ..] -> Ok(Up)
            [5, ..] -> Ok(Down)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    test_roundtrip_for_variant(2, 1, 0)?
    test_roundtrip_for_variant(3, 1, 0)?
    test_roundtrip_for_variant(4, 1, 0)?
    test_roundtrip_for_variant(5, 1, 0)?
    Ok({})

test_roundtrip_for_variant : U8, U64, U64 -> Result {} _
test_roundtrip_for_variant = |discriminant, variant_size, padding_size|
    bytes = 
        List.range({ start: At discriminant, end: Length variant_size })
        |> List.concat(List.repeat(0, padding_size))
        |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
