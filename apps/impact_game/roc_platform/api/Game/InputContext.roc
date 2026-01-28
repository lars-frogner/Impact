# Hash: 1d99c1cf36642d34
# Generated: 2026-01-25T13:02:32.838920337
# Rust type: impact_game::input::InputContext
# Type category: Inline
module [
    InputContext,
    write_bytes,
    from_bytes,
]

import Game.InteractionMode

InputContext : {
    interaction_mode : Game.InteractionMode.InteractionMode,
}

## Serializes a value of [InputContext] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InputContext -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(1)
    |> Game.InteractionMode.write_bytes(value.interaction_mode)

## Deserializes a value of [InputContext] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InputContext _
from_bytes = |bytes|
    Ok(
        {
            interaction_mode: bytes |> List.sublist({ start: 0, len: 1 }) |> Game.InteractionMode.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 1 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
