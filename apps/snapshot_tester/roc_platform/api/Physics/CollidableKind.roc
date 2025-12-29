# Hash: b2a9a4311524930d
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_physics::collision::CollidableKind
# Type category: Inline
module [
    CollidableKind,
    write_bytes,
    from_bytes,
]

CollidableKind : [
    Dynamic,
    Static,
    Phantom,
]

## Serializes a value of [CollidableKind] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CollidableKind -> List U8
write_bytes = |bytes, value|
    when value is
        Dynamic ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Static ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Phantom ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [CollidableKind] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CollidableKind _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Dynamic)
            [1, ..] -> Ok(Static)
            [2, ..] -> Ok(Phantom)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
