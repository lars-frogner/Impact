# Hash: be8324e7455bbb58ee1f72890d6f1b418b37fddb95c09acc1b330bcebb2573a4
# Generated: 2025-07-15T17:32:17+00:00
# Rust type: impact::window::input::key::LockKey
# Type category: Inline
# Commit: 1fbb6f6b (dirty)
module [
    LockKey,
    write_bytes,
    from_bytes,
]

LockKey : [
    CapsLock,
    NumLock,
]

## Serializes a value of [LockKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LockKey -> List U8
write_bytes = |bytes, value|
    when value is
        CapsLock ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        NumLock ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [LockKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LockKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(CapsLock)
            [1, ..] -> Ok(NumLock)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
