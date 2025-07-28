# Hash: 1ed19409e95ff5b95685b8f109bcfb7ef6d9022b8defa89289c67f3503f979ec
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact_controller::motion::MotionDirection
# Type category: Inline
# Commit: 397d36d3 (dirty)
module [
    MotionDirection,
    write_bytes,
    from_bytes,
]

## Possible directions of motion in the local coordinate system.
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
