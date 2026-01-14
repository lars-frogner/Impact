# Hash: afd62fe3795bef6b
# Generated: 2026-01-14T12:38:27.753362153
# Rust type: impact::command::physics::LocalForceUpdateMode
# Type category: Inline
module [
    LocalForceUpdateMode,
    write_bytes,
    from_bytes,
]

LocalForceUpdateMode : [
    Set,
    Add,
]

## Serializes a value of [LocalForceUpdateMode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LocalForceUpdateMode -> List U8
write_bytes = |bytes, value|
    when value is
        Set ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Add ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [LocalForceUpdateMode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LocalForceUpdateMode _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Set)
            [1, ..] -> Ok(Add)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
