# Hash: 39c433d364a13d83c87498b9f4544eea1d049b305f7ca649cbc067fba62f0755
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact_dev_ui::command::UICommand
# Type category: Inline
# Commit: 189570ab (dirty)
module [
    UICommand,
    write_bytes,
    from_bytes,
]

import Command.ToActiveState

UICommand : [
    SetInteractivity Command.ToActiveState.ToActiveState,
]

## Serializes a value of [UICommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UICommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetInteractivity(val) ->
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
                    SetInteractivity(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
