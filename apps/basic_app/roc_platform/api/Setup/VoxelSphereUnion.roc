# Hash: 1bd121f80a7464bb
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact_voxel::setup::VoxelSphereUnion
# Type category: Component
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

## An object made of voxels in a configuration described by the smooth
## union of two spheres.
VoxelSphereUnion : {
    ## The extent of a single voxel.
    voxel_extent : F32,
    ## The number of voxels along the radius of the first sphere.
    radius_1 : F32,
    ## The number of voxels along the radius of the second sphere.
    radius_2 : F32,
    ## The offset in number of voxels in each dimension between the centers of
    ## the two spheres.
    center_offsets : Vector3.Vector3,
    ## The smoothness of the union operation.
    smoothness : F32,
}

## Defines a sphere union with the given smoothness of the spheres with the
## given radii and center offsets (in voxels).
##
## # Panics
## - If the voxel extent is negative.
## - If either of the radii is zero or negative.
new : F32, F32, F32, Vector3.Vector3, F32 -> VoxelSphereUnion
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

## Defines a sphere union with the given smoothness of the spheres with the
## given radii and center offsets (in voxels).
##
## # Panics
## - If the voxel extent is negative.
## - If either of the radii is zero or negative.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, F32, F32, F32, Vector3.Vector3, F32 -> Entity.ComponentData
add_new = |entity_data, voxel_extent, radius_1, radius_2, center_offsets, smoothness|
    add(entity_data, new(voxel_extent, radius_1, radius_2, center_offsets, smoothness))

## Adds a value of the [VoxelSphereUnion] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, VoxelSphereUnion -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelSphereUnion] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (VoxelSphereUnion) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelSphereUnion.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelSphereUnion -> List U8
write_packet = |bytes, val|
    type_id = 10023429030278991225
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelSphereUnion -> List U8
write_multi_packet = |bytes, vals|
    type_id = 10023429030278991225
    size = 28
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

## Serializes a value of [VoxelSphereUnion] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelSphereUnion -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(28)
    |> Builtin.write_bytes_f32(value.voxel_extent)
    |> Builtin.write_bytes_f32(value.radius_1)
    |> Builtin.write_bytes_f32(value.radius_2)
    |> Vector3.write_bytes(value.center_offsets)
    |> Builtin.write_bytes_f32(value.smoothness)

## Deserializes a value of [VoxelSphereUnion] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelSphereUnion _
from_bytes = |bytes|
    Ok(
        {
            voxel_extent: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            radius_1: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            radius_2: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            center_offsets: bytes |> List.sublist({ start: 12, len: 12 }) |> Vector3.from_bytes?,
            smoothness: bytes |> List.sublist({ start: 24, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 28 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
