# Hash: f323255714be9895
# Generated: 2026-01-16T08:39:55.238376309
# Rust type: impact_physics::force::alignment_torque::AlignmentDirection
# Type category: Inline
module [
    AlignmentDirection,
    write_bytes,
    from_bytes,
]

import core.UnitVector3

## An external direction a body can be aligned with.
AlignmentDirection : [
    Fixed UnitVector3.UnitVector3,
    GravityForce,
]

## Serializes a value of [AlignmentDirection] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AlignmentDirection -> List U8
write_bytes = |bytes, value|
    when value is
        Fixed(val) ->
            bytes
            |> List.reserve(13)
            |> List.append(0)
            |> UnitVector3.write_bytes(val)

        GravityForce ->
            bytes
            |> List.reserve(13)
            |> List.append(1)
            |> List.concat(List.repeat(0, 12))

## Deserializes a value of [AlignmentDirection] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AlignmentDirection _
from_bytes = |bytes|
    if List.len(bytes) != 13 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    Fixed(
                        data_bytes |> List.sublist({ start: 0, len: 12 }) |> UnitVector3.from_bytes?,
                    ),
                )

            [1, ..] -> Ok(GravityForce)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
