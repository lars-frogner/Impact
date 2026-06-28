# Hash: 4aa1adc61148a38e
# Generated: 2026-06-27T22:09:06.626391794
# Rust type: basic_app::command::AppCommand
# Type category: Inline
module [
    AppCommand,
    write_bytes,
    from_bytes,
]

import Entity
import core.Builtin

AppCommand : [
    FractureVoxelObject {
            entity_id : Entity.Id,
            points_per_dim : U64,
        },
]

## Serializes a value of [AppCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AppCommand -> List U8
write_bytes = |bytes, value|
    when value is
        FractureVoxelObject { entity_id, points_per_dim } ->
            bytes
            |> List.reserve(17)
            |> List.append(0)
            |> Entity.write_bytes_id(entity_id)
            |> Builtin.write_bytes_u64(points_per_dim)

## Deserializes a value of [AppCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AppCommand _
from_bytes = |bytes|
    if List.len(bytes) != 17 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    FractureVoxelObject     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        points_per_dim: data_bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_u64?,
                    },
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
