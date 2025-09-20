# Hash: 5aaa193c8f9118bf6cc6c5f953e13cbad7010cb5a41f3196f90945aa11a05720
# Generated: 2025-09-20T12:42:00+00:00
# Rust type: impact_mesh::setup::PlanarTextureProjection
# Type category: Component
# Commit: f9b55709 (dirty)
module [
    PlanarTextureProjection,
    new,
    for_rectangle,
    add_new,
    add_multiple_new,
    add_for_rectangle,
    add_multiple_for_rectangle,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Setup.RectangleMesh
import core.Builtin
import core.Point3
import core.Vector3

## The properties of a
## [`PlanarTextureProjection`](crate::texture_projection::PlanarTextureProjection).
PlanarTextureProjection : {
    ## The origin of the plane, where the texture coordinates will be zero.
    origin : Point3.Point3 Binary32,
    ## The axis along which the U texture coordinate will increase. The texture
    ## coordinate will be unity at the tip of the vector.
    u_vector : Vector3.Vector3 Binary32,
    ## The axis along which the V texture coordinate will increase. The texture
    ## coordinate will be unity at the tip of the vector.
    v_vector : Vector3.Vector3 Binary32,
}

## Creates the properties of a projection onto the plane defined by the
## given origin and two vectors defining the axes along which the U and V
## texture coordinates will increase. The texture coordinates will be zero
## at the origin and unity at the tip of the respective u- or v-vector.
new : Point3.Point3 Binary32, Vector3.Vector3 Binary32, Vector3.Vector3 Binary32 -> PlanarTextureProjection
new = |origin, u_vector, v_vector|
    { origin, u_vector, v_vector }

## Creates the properties of a projection onto the plane defined by the
## given origin and two vectors defining the axes along which the U and V
## texture coordinates will increase. The texture coordinates will be zero
## at the origin and unity at the tip of the respective u- or v-vector.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, Point3.Point3 Binary32, Vector3.Vector3 Binary32, Vector3.Vector3 Binary32 -> Entity.ComponentData
add_new = |entity_data, origin, u_vector, v_vector|
    add(entity_data, new(origin, u_vector, v_vector))

## Creates the properties of a projection onto the plane defined by the
## given origin and two vectors defining the axes along which the U and V
## texture coordinates will increase. The texture coordinates will be zero
## at the origin and unity at the tip of the respective u- or v-vector.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Point3.Point3 Binary32), Entity.Arg.Broadcasted (Vector3.Vector3 Binary32), Entity.Arg.Broadcasted (Vector3.Vector3 Binary32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, origin, u_vector, v_vector|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin, u_vector, v_vector,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Creates the properties of a projection onto the axis-aligned horizontal
## rectangle specified by the given [`RectangleMesh`], scaling the
## projection so that the texture will repeat the given numbers of times
## along the U and V texture coordinate directions. The U-axis will be
## aligned with the x-axis and the V-axis will be aligned with the negative
## z-axis.
for_rectangle : Setup.RectangleMesh.RectangleMesh, F32, F32 -> PlanarTextureProjection
for_rectangle = |rectangle, n_repeats_u, n_repeats_v|
    origin = (-0.5, 0.0, 0.5)
    u_vector = (rectangle.extent_x / n_repeats_u, 0.0, 0.0)
    v_vector = (0.0, 0.0, -rectangle.extent_z / n_repeats_v)
    new(origin, u_vector, v_vector)

## Creates the properties of a projection onto the axis-aligned horizontal
## rectangle specified by the given [`RectangleMesh`], scaling the
## projection so that the texture will repeat the given numbers of times
## along the U and V texture coordinate directions. The U-axis will be
## aligned with the x-axis and the V-axis will be aligned with the negative
## z-axis.
## Adds the component to the given entity's data.
add_for_rectangle : Entity.ComponentData, Setup.RectangleMesh.RectangleMesh, F32, F32 -> Entity.ComponentData
add_for_rectangle = |entity_data, rectangle, n_repeats_u, n_repeats_v|
    add(entity_data, for_rectangle(rectangle, n_repeats_u, n_repeats_v))

## Creates the properties of a projection onto the axis-aligned horizontal
## rectangle specified by the given [`RectangleMesh`], scaling the
## projection so that the texture will repeat the given numbers of times
## along the U and V texture coordinate directions. The U-axis will be
## aligned with the x-axis and the V-axis will be aligned with the negative
## z-axis.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_rectangle : Entity.MultiComponentData, Entity.Arg.Broadcasted (Setup.RectangleMesh.RectangleMesh), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiComponentData Str
add_multiple_for_rectangle = |entity_data, rectangle, n_repeats_u, n_repeats_v|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            rectangle, n_repeats_u, n_repeats_v,
            Entity.multi_count(entity_data),
            for_rectangle
        ))
    )

## Adds a value of the [PlanarTextureProjection] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, PlanarTextureProjection -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [PlanarTextureProjection] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (PlanarTextureProjection) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in PlanarTextureProjection.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, PlanarTextureProjection -> List U8
write_packet = |bytes, val|
    type_id = 1060868192652842756
    size = 36
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List PlanarTextureProjection -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1060868192652842756
    size = 36
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

## Serializes a value of [PlanarTextureProjection] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PlanarTextureProjection -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(36)
    |> Point3.write_bytes_32(value.origin)
    |> Vector3.write_bytes_32(value.u_vector)
    |> Vector3.write_bytes_32(value.v_vector)

## Deserializes a value of [PlanarTextureProjection] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PlanarTextureProjection _
from_bytes = |bytes|
    Ok(
        {
            origin: bytes |> List.sublist({ start: 0, len: 12 }) |> Point3.from_bytes_32?,
            u_vector: bytes |> List.sublist({ start: 12, len: 12 }) |> Vector3.from_bytes_32?,
            v_vector: bytes |> List.sublist({ start: 24, len: 12 }) |> Vector3.from_bytes_32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 36 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
