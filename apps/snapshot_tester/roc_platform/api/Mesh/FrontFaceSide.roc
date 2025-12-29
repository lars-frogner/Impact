# Hash: 63e97900fd7803c8
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_mesh::FrontFaceSide
# Type category: Inline
module [
    FrontFaceSide,
    write_bytes,
    from_bytes,
]

## Whether the front faces of a triangle mesh are oriented toward the outside
## or the inside.
FrontFaceSide : [
    Outside,
    Inside,
]

## Serializes a value of [FrontFaceSide] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FrontFaceSide -> List U8
write_bytes = |bytes, value|
    when value is
        Outside ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Inside ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [FrontFaceSide] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FrontFaceSide _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Outside)
            [1, ..] -> Ok(Inside)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
