# Hash: f007df5406cfb7f0f1dfa0c0307fba43f2246fd774a32714f5fa3d00640d1215
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact::voxel::components::VoxelAbsorbingSphereComp
# Type category: Component
# Commit: ce2d27b (dirty)
module [
    VoxelAbsorbingSphere,
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
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that have a
## sphere that absorbs voxels it comes in contact with. The rate of absorption
## is highest at the center of the sphere and decreases quadratically to zero
## at the full radius.
##
## Does nothing if the entity does not have a
## [`crate::physics::motion::components::ReferenceFrameComp`].
VoxelAbsorbingSphere : {
    ## The offset of the sphere in the reference frame of the entity.
    offset : Vector3.Vector3 Binary64,
    ## The radius of the sphere.
    radius : F64,
    ## The maximum rate of absorption (at the center of the sphere).
    rate : F64,
}

## Creates a new [`VoxelAbsorbingSphereComp`] with the given offset and
## radius in the reference frame of the entity and the given maximum
## absorption rate (at the center of the sphere).
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

## Creates a new [`VoxelAbsorbingSphereComp`] with the given offset and
## radius in the reference frame of the entity and the given maximum
## absorption rate (at the center of the sphere).
## Adds the component to the given entity's data.
add_new : Entity.Data, Vector3.Vector3 Binary64, F64, F64 -> Entity.Data
add_new = |entity_data, offset, radius, rate|
    add(entity_data, new(offset, radius, rate))

## Creates a new [`VoxelAbsorbingSphereComp`] with the given offset and
## radius in the reference frame of the entity and the given maximum
## absorption rate (at the center of the sphere).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
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
add : Entity.Data, VoxelAbsorbingSphere -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelAbsorbingSphere] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (VoxelAbsorbingSphere) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelAbsorbingSphere.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelAbsorbingSphere -> List U8
write_packet = |bytes, val|
    type_id = 13339386110717950888
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
    type_id = 13339386110717950888
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
