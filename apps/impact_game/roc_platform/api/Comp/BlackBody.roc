# Hash: 2fccd70488de1ec8
# Generated: 2026-02-06T11:53:57.523304204
# Rust type: impact_game::entities::black_body::BlackBody
# Type category: Component
module [
    BlackBody,
    new,
    add_new,
    add_multiple_new,
    sphere,
    add_sphere,
    add,
    add_multiple,
    component_id,
    add_component_id,
    read,
    get_for_entity!,
    set_for_entity!,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

BlackBody : {
    surface_area : F32,
    heat_capacity : F32,
    emissivity : F32,
    temperature : F32,
}

new : F32, F32, F32, F32 -> BlackBody
new = |surface_area, heat_capacity, emissivity, temperature|
    { surface_area, heat_capacity, emissivity, temperature }

add_new : Entity.ComponentData, F32, F32, F32, F32 -> Entity.ComponentData
add_new = |entity_data, surface_area, heat_capacity, emissivity, temperature|
    add(entity_data, new(surface_area, heat_capacity, emissivity, temperature))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, surface_area, heat_capacity, emissivity, temperature|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            surface_area, heat_capacity, emissivity, temperature,
            Entity.multi_count(entity_data),
            new
        ))
    )

sphere : F32, F32, F32, F32, F32 -> BlackBody
sphere = |radius, mass, specific_heat_capacity, emissivity, temperature|
    surface_area = 4 * Num.pi * radius * radius
    heat_capacity = specific_heat_capacity * mass
    { surface_area, heat_capacity, emissivity, temperature }

add_sphere : Entity.ComponentData, F32, F32, F32, F32, F32 -> Entity.ComponentData
add_sphere = |entity_data, radius, mass, specific_heat_capacity, emissivity, temperature|
    add(entity_data, sphere(radius, mass, specific_heat_capacity, emissivity, temperature))

## Adds a value of the [BlackBody] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, BlackBody -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [BlackBody] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (BlackBody) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in BlackBody.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [BlackBody] component.
component_id = 13883484099747743208

## Adds the ID of the [BlackBody] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result BlackBody Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No BlackBody component in data"
                Decode(decode_err) -> "Failed to decode BlackBody component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result BlackBody Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : BlackBody, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, BlackBody -> List U8
write_packet = |bytes, val|
    type_id = 13883484099747743208
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List BlackBody -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13883484099747743208
    size = 16
    alignment = 4
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

## Serializes a value of [BlackBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, BlackBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f32(value.surface_area)
    |> Builtin.write_bytes_f32(value.heat_capacity)
    |> Builtin.write_bytes_f32(value.emissivity)
    |> Builtin.write_bytes_f32(value.temperature)

## Deserializes a value of [BlackBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result BlackBody _
from_bytes = |bytes|
    Ok(
        {
            surface_area: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            heat_capacity: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            emissivity: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            temperature: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
