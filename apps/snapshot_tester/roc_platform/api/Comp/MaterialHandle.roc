# Hash: 03a03d9f738addb5c2b7b9bbeb31d68a0be90012ac38a5ef796d3afe94571b9d
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact_material::MaterialHandle
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    MaterialHandle,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Material.MaterialID
import Material.MaterialPropertyTextureGroupID
import Model.InstanceFeatureID
import core.Builtin

## A handle for a material, containing the IDs for the pieces of data holding
## information about the material.
MaterialHandle : {
    ## The ID of the material's [`MaterialSpecification`].
    material_id : Material.MaterialID.MaterialID,
    ## The ID of the entry for the material's per-instance material properties
    ## in the [`InstanceFeatureStorage`](impact_model::InstanceFeatureStorage)
    ## (may be N/A).
    material_property_feature_id : Model.InstanceFeatureID.InstanceFeatureID,
    ## The ID of the material's [`MaterialPropertyTextureGroup`] (may represent
    ## an empty group).
    material_property_texture_group_id : Material.MaterialPropertyTextureGroupID.MaterialPropertyTextureGroupID,
}

## Adds a value of the [MaterialHandle] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, MaterialHandle -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [MaterialHandle] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (MaterialHandle) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in MaterialHandle.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, MaterialHandle -> List U8
write_packet = |bytes, val|
    type_id = 16835469039925593474
    size = 32
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List MaterialHandle -> List U8
write_multi_packet = |bytes, vals|
    type_id = 16835469039925593474
    size = 32
    alignment = 8
    count = List.len(vals)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    vals
    |> List.walk(
        bytes_with_header,
        |bts, value| bts |> write_bytes(value),
    )

## Serializes a value of [MaterialHandle] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MaterialHandle -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Material.MaterialID.write_bytes(value.material_id)
    |> Model.InstanceFeatureID.write_bytes(value.material_property_feature_id)
    |> Material.MaterialPropertyTextureGroupID.write_bytes(value.material_property_texture_group_id)

## Deserializes a value of [MaterialHandle] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MaterialHandle _
from_bytes = |bytes|
    Ok(
        {
            material_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Material.MaterialID.from_bytes?,
            material_property_feature_id: bytes |> List.sublist({ start: 8, len: 16 }) |> Model.InstanceFeatureID.from_bytes?,
            material_property_texture_group_id: bytes |> List.sublist({ start: 24, len: 8 }) |> Material.MaterialPropertyTextureGroupID.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 32 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
