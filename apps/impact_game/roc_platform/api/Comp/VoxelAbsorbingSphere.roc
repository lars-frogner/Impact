# Hash: c3b387c7f371ee78f638ba01e2d12bf06559ff33dff80e0bb96369e253fa1e06
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::voxel::components::VoxelAbsorbingSphereComp
# Type category: Component
# Commit: d505d37
module [
    VoxelAbsorbingSphere,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that have a
## sphere that absorbs voxels it comes in contact with. The rate of absorption
## is highest at the center of the sphere and decreases quadratically to zero
## at the full radius.
##
## Does nothing if the entity does not have a [`ReferenceFrameComp`].
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
    expect radius >= 0.0
    expect rate >= 0.0
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
add_new = |data, offset, radius, rate|
    add(data, new(offset, radius, rate))

## Adds a value of the [VoxelAbsorbingSphere] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelAbsorbingSphere -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [VoxelAbsorbingSphere] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List VoxelAbsorbingSphere -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, VoxelAbsorbingSphere -> List U8
write_packet = |bytes, value|
    type_id = 13339386110717950888
    size = 40
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List VoxelAbsorbingSphere -> List U8
write_multi_packet = |bytes, values|
    type_id = 13339386110717950888
    size = 40
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
