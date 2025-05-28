# Hash: 8c8495f636b4d9523ca2421807785b3f1fbb36727efad4016e8cccc0d04680f1
# Generated: 2025-05-28T20:09:22+00:00
# Rust type: impact::ui::command::UICommand
# Type category: Inline
# Commit: ff9febb (dirty)
module [
    UICommand,
    write_bytes,
    from_bytes,
]

import Command.ToActiveState

UICommand : [
    Set Command.ToActiveState.ToActiveState,
]

## Serializes a value of [UICommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UICommand -> List U8
write_bytes = |bytes, value|
    when value is
        Set(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> Command.ToActiveState.write_bytes(val)

## Deserializes a value of [UICommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UICommand _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    Set(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
