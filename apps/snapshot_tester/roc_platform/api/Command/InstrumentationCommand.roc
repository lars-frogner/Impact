# Hash: 45aae28fa4a1ab9166a2da5bedfceb6eac44a8e578e5ac265f2feba90d2ee7fe
# Generated: 2025-07-15T17:32:43+00:00
# Rust type: impact::instrumentation::command::InstrumentationCommand
# Type category: Inline
# Commit: 1fbb6f6b (dirty)
module [
    InstrumentationCommand,
    write_bytes,
    from_bytes,
]

import Command.ToActiveState

InstrumentationCommand : [
    SetTaskTimings Command.ToActiveState.ToActiveState,
]

## Serializes a value of [InstrumentationCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InstrumentationCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetTaskTimings(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> Command.ToActiveState.write_bytes(val)

## Deserializes a value of [InstrumentationCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InstrumentationCommand _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetTaskTimings(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
