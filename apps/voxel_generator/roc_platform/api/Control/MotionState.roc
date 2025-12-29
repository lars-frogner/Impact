# Hash: abf7d2867c1fd4f4
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_controller::motion::MotionState
# Type category: Inline
module [
    MotionState,
    write_bytes,
    from_bytes,
]

## Whether there is motion in a certain direction.
MotionState : [
    Still,
    Moving,
]

## Serializes a value of [MotionState] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MotionState -> List U8
write_bytes = |bytes, value|
    when value is
        Still ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Moving ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [MotionState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MotionState _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Still)
            [1, ..] -> Ok(Moving)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
