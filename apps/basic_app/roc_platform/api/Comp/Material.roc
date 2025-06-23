# Hash: fb1c963ddc5738d618b2bf0f56ecbda1964506ec4b6942720124d7e6620cf50b
# Generated: 2025-06-23T21:05:32+00:00
# Rust type: impact_material::components::MaterialComp
# Type category: Component
# Commit: 6a2f327 (dirty)
module [
    Material,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Material.MaterialHandle
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that
## have a material.
Material : {
    material_handle : Material.MaterialHandle.MaterialHandle,
}

## Creates a new component representing the material with the given handle.
new : Material.MaterialHandle.MaterialHandle -> Material
new = |material_handle|
    { material_handle }

## Creates a new component representing the material with the given handle.
## Adds the component to the given entity's data.
add_new : Entity.Data, Material.MaterialHandle.MaterialHandle -> Entity.Data
add_new = |entity_data, material_handle|
    add(entity_data, new(material_handle))

## Creates a new component representing the material with the given handle.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Material.MaterialHandle.MaterialHandle) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, material_handle|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            material_handle,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [Material] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Material -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [Material] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (Material) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in Material.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, Material -> List U8
write_packet = |bytes, val|
    type_id = 5599710697835304399
    size = 32
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List Material -> List U8
write_multi_packet = |bytes, vals|
    type_id = 5599710697835304399
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

## Serializes a value of [Material] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Material -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Material.MaterialHandle.write_bytes(value.material_handle)

## Deserializes a value of [Material] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Material _
from_bytes = |bytes|
    Ok(
        {
            material_handle: bytes |> List.sublist({ start: 0, len: 32 }) |> Material.MaterialHandle.from_bytes?,
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
