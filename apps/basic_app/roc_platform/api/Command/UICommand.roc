# Hash: 4a74e53ba0654979d6241a10b428187c9b927b687c810c34fb4c0612d5f6c9e6
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::ui::command::UICommand
# Type category: Inline
# Commit: 31f3514 (dirty)
module [
    UICommand,
    write_bytes,
    from_bytes,
]

import Command.ToInteractionMode

UICommand : [
    SetInteractionMode Command.ToInteractionMode.ToInteractionMode,
]

## Serializes a value of [UICommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UICommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetInteractionMode(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> Command.ToInteractionMode.write_bytes(val)

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
                    SetInteractionMode(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToInteractionMode.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
