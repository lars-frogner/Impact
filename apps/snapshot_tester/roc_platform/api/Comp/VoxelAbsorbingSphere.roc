# Hash: 200a2d8ecd6827c98dd3828c45a5db59e64bbe52196ed406425485bb3917cc1c
# Generated: 2025-09-20T12:42:00+00:00
# Rust type: impact_voxel::interaction::absorption::VoxelAbsorbingSphere
# Type category: Component
# Commit: f9b55709 (dirty)
module [
    VoxelAbsorbingSphere,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    component_id,
    add_component_id,
    read,
    get_for_entity!,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Vector3

## A sphere that absorbs voxels it comes in contact with. The rate of
## absorption is highest at the center of the sphere and decreases
## quadratically to zero at the full radius.
##
## Does nothing if the entity does not have a
## [`impact_geometry::ReferenceFrame`].
VoxelAbsorbingSphere : {
    ## The offset of the sphere in the reference frame of the entity.
    offset : Vector3.Vector3 Binary64,
    ## The radius of the sphere.
    radius : F64,
    ## The maximum rate of absorption (at the center of the sphere).
    rate : F64,
}

## Creates a new [`VoxelAbsorbingSphere`] with the given offset and radius
## in the reference frame of the entity and the given maximum absorption
## rate (at the center of the sphere).
new : Vector3.Vector3 Binary64, F64, F64 -> VoxelAbsorbingSphere
new = |offset, radius, rate|
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    # expect rate >= 0.0
    {
        offset,
        radius,
        rate,
    }

## Creates a new [`VoxelAbsorbingSphere`] with the given offset and radius
## in the reference frame of the entity and the given maximum absorption
## rate (at the center of the sphere).
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, Vector3.Vector3 Binary64, F64, F64 -> Entity.ComponentData
add_new = |entity_data, offset, radius, rate|
    add(entity_data, new(offset, radius, rate))

## Creates a new [`VoxelAbsorbingSphere`] with the given offset and radius
## in the reference frame of the entity and the given maximum absorption
## rate (at the center of the sphere).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, offset, radius, rate|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            offset, radius, rate,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [VoxelAbsorbingSphere] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, VoxelAbsorbingSphere -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelAbsorbingSphere] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (VoxelAbsorbingSphere) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelAbsorbingSphere.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [VoxelAbsorbingSphere] component.
component_id = 13800759532896143647

## Adds the ID of the [VoxelAbsorbingSphere] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result VoxelAbsorbingSphere Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No VoxelAbsorbingSphere component in data"
                Decode(decode_err) -> "Failed to decode VoxelAbsorbingSphere component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result VoxelAbsorbingSphere Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

write_packet : List U8, VoxelAbsorbingSphere -> List U8
write_packet = |bytes, val|
    type_id = 13800759532896143647
    size = 40
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelAbsorbingSphere -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13800759532896143647
    size = 40
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

## Serializes a value of [VoxelAbsorbingSphere] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelAbsorbingSphere -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(40)
    |> Vector3.write_bytes_64(value.offset)
    |> Builtin.write_bytes_f64(value.radius)
    |> Builtin.write_bytes_f64(value.rate)

## Deserializes a value of [VoxelAbsorbingSphere] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelAbsorbingSphere _
from_bytes = |bytes|
    Ok(
        {
            offset: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            radius: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
            rate: bytes |> List.sublist({ start: 32, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 40 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
