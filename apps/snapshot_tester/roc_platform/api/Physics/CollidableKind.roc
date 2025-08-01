# Hash: 49eb2f100b6121bae3723c476bb39cf01d6d0f15f67dc69d382170c52711a42c
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact_physics::collision::CollidableKind
# Type category: Inline
# Commit: 397d36d3 (dirty)
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
