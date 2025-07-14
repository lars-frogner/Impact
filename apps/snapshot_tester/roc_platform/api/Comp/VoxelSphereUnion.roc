# Hash: 957cd0f3fed57fd3af9eaec94dc946d0c995784af2728a34defd9d44ce1fa9af
# Generated: 2025-07-13T20:18:37+00:00
# Rust type: impact::voxel::components::VoxelSphereUnionComp
# Type category: Component
# Commit: b1b4dfd8 (dirty)
module [
    VoxelSphereUnion,
    new,
    add_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Vector3

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities comprised of voxels in a configuration described by the smooth
## union of two spheres.
##
## The purpose of this component is to aid in constructing a
## [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
## creation.
VoxelSphereUnion : {
    ## The extent of a single voxel.
    voxel_extent : F64,
    ## The number of voxels along the radius of the first sphere.
    radius_1 : F64,
    ## The number of voxels along the radius of the second sphere.
    radius_2 : F64,
    ## The offset in number of voxels in each dimension between the centers of
    ## the two spheres.
    center_offsets : Vector3.Vector3 Binary64,
    ## The smoothness of the union operation.
    smoothness : F64,
}

## Creates a new component for a sphere union with the given smoothness of
## the spheres with the given radii and center offsets (in voxels).
##
## # Panics
## - If the voxel extent is negative.
## - If either of the radii is zero or negative.
new : F64, F64, F64, Vector3.Vector3 Binary64, F64 -> VoxelSphereUnion
new = |voxel_extent, radius_1, radius_2, center_offsets, smoothness|
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect radius_1 >= 0.0
    # expect radius_2 >= 0.0
    {
        voxel_extent,
        radius_1,
        radius_2,
        center_offsets,
        smoothness,
    }

## Creates a new component for a sphere union with the given smoothness of
## the spheres with the given radii and center offsets (in voxels).
##
## # Panics
## - If the voxel extent is negative.
## - If either of the radii is zero or negative.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, F64, F64, Vector3.Vector3 Binary64, F64 -> Entity.Data
add_new = |entity_data, voxel_extent, radius_1, radius_2, center_offsets, smoothness|
    add(entity_data, new(voxel_extent, radius_1, radius_2, center_offsets, smoothness))

## Adds a value of the [VoxelSphereUnion] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelSphereUnion -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelSphereUnion] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (VoxelSphereUnion) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelSphereUnion.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelSphereUnion -> List U8
write_packet = |bytes, val|
    type_id = 15024179413351373586
    size = 56
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelSphereUnion -> List U8
write_multi_packet = |bytes, vals|
    type_id = 15024179413351373586
    size = 56
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

## Serializes a value of [VoxelSphereUnion] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelSphereUnion -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Builtin.write_bytes_f64(value.voxel_extent)
    |> Builtin.write_bytes_f64(value.radius_1)
    |> Builtin.write_bytes_f64(value.radius_2)
    |> Vector3.write_bytes_64(value.center_offsets)
    |> Builtin.write_bytes_f64(value.smoothness)

## Deserializes a value of [VoxelSphereUnion] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelSphereUnion _
from_bytes = |bytes|
    Ok(
        {
            voxel_extent: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            radius_1: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            radius_2: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
            center_offsets: bytes |> List.sublist({ start: 24, len: 24 }) |> Vector3.from_bytes_64?,
            smoothness: bytes |> List.sublist({ start: 48, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 56 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
