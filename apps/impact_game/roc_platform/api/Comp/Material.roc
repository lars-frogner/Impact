# Hash: 3b0e967a1e1bd91830bc0a3e27aaf2b845cf01eb7afde0098e65d7381bbac74f
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::MaterialComp
# Type category: Component
# Commit: d505d37
module [
    Material,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
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
add_new = |data, material_handle|
    add(data, new(material_handle))

## Adds a value of the [Material] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Material -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [Material] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List Material -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, Material -> List U8
write_packet = |bytes, value|
    type_id = 12873697595636024364
    size = 32
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List Material -> List U8
write_multi_packet = |bytes, values|
    type_id = 12873697595636024364
    size = 32
    alignment = 8
    count = List.len(values)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    values
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
