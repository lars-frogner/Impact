# Hash: 888e927451bc436120bd5419dbd3f197d08202aaa0ddc77227fbcf23ac57e7d0
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::voxel::components::VoxelAbsorbingCapsuleComp
# Type category: Component
# Commit: d505d37
module [
    VoxelAbsorbingCapsule,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that have a
## capsule that absorbs voxels it comes in contact with. The rate of absorption
## is highest at the central line segment of the capsule and decreases
## quadratically to zero at the capsule boundary.
##
## Does nothing if the entity does not have a [`ReferenceFrameComp`].
VoxelAbsorbingCapsule : {
    ## The offset of the starting point of the capsule's central line segment
    ## in the reference frame of the entity.
    offset_to_segment_start : Vector3.Vector3 Binary64,
    ## The displacement vector from the start to the end of the capsule's
    ## central line segment in the reference frame of the entity.
    segment_vector : Vector3.Vector3 Binary64,
    ## The radius of the capsule.
    radius : F64,
    ## The maximum rate of absorption (at the central line segment of the
    ## capsule).
    rate : F64,
}

## Creates a new [`VoxelAbsorbingCapsuleComp`] with the given offset to the
## start of the capsule's central line segment, displacement from the start
## to the end of the line segment and radius, all in the reference frame of
## the entity, as well as the given maximum absorption rate (at the central
## line segment).
new : Vector3.Vector3 Binary64, Vector3.Vector3 Binary64, F64, F64 -> VoxelAbsorbingCapsule
new = |offset_to_segment_start, segment_vector, radius, rate|
    expect radius >= 0.0
    expect rate >= 0.0
    {
        offset_to_segment_start,
        segment_vector,
        radius,
        rate,
    }

## Creates a new [`VoxelAbsorbingCapsuleComp`] with the given offset to the
## start of the capsule's central line segment, displacement from the start
## to the end of the line segment and radius, all in the reference frame of
## the entity, as well as the given maximum absorption rate (at the central
## line segment).
## Adds the component to the given entity's data.
add_new : Entity.Data, Vector3.Vector3 Binary64, Vector3.Vector3 Binary64, F64, F64 -> Entity.Data
add_new = |data, offset_to_segment_start, segment_vector, radius, rate|
    add(data, new(offset_to_segment_start, segment_vector, radius, rate))

## Adds a value of the [VoxelAbsorbingCapsule] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelAbsorbingCapsule -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [VoxelAbsorbingCapsule] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List VoxelAbsorbingCapsule -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, VoxelAbsorbingCapsule -> List U8
write_packet = |bytes, value|
    type_id = 7356672927394480030
    size = 64
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List VoxelAbsorbingCapsule -> List U8
write_multi_packet = |bytes, values|
    type_id = 7356672927394480030
    size = 64
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

## Serializes a value of [VoxelAbsorbingCapsule] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelAbsorbingCapsule -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(64)
    |> Vector3.write_bytes_64(value.offset_to_segment_start)
    |> Vector3.write_bytes_64(value.segment_vector)
    |> Builtin.write_bytes_f64(value.radius)
    |> Builtin.write_bytes_f64(value.rate)

## Deserializes a value of [VoxelAbsorbingCapsule] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelAbsorbingCapsule _
from_bytes = |bytes|
    Ok(
        {
            offset_to_segment_start: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            segment_vector: bytes |> List.sublist({ start: 24, len: 24 }) |> Vector3.from_bytes_64?,
            radius: bytes |> List.sublist({ start: 48, len: 8 }) |> Builtin.from_bytes_f64?,
            rate: bytes |> List.sublist({ start: 56, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 64 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
