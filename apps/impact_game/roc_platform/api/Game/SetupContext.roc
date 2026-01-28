# Hash: dc3b3ce3bfc64476
# Generated: 2026-01-25T13:02:32.838920337
# Rust type: impact_game::setup::SetupContext
# Type category: Inline
module [
    SetupContext,
    write_bytes,
    from_bytes,
]

import Game.InteractionMode

SetupContext : {
    interaction_mode : Game.InteractionMode.InteractionMode,
}

## Serializes a value of [SetupContext] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SetupContext -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(1)
    |> Game.InteractionMode.write_bytes(value.interaction_mode)

## Deserializes a value of [SetupContext] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SetupContext _
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
