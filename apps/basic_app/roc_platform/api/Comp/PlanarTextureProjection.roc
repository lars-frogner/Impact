# Hash: 9e248325f4312280a63910402dbf9667da91cf0070aa1d875f389bf98a022a10
# Generated: 2025-05-23T21:48:57+00:00
# Rust type: impact::mesh::texture_projection::components::PlanarTextureProjectionComp
# Type category: Component
# Commit: 31f3514 (dirty)
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

import Comp.RectangleMesh
import Entity
import Entity.Arg
import core.Builtin
import core.Point3
import core.Vector3

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that use a [`PlanarTextureProjection`].
##
## The purpose of this component is to aid in constructing a
## [`MeshComp`](crate::mesh::components::MeshComp) for the entity. It is
## therefore not kept after entity creation.
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

## Creates the component for a projection onto the plane defined by the
## given origin and two vectors defining the axes along which the U and V
## texture coordinates will increase. The texture coordinates will be zero
## at the origin and unity at the tip of the respective u- or v-vector.
new : Point3.Point3 Binary32, Vector3.Vector3 Binary32, Vector3.Vector3 Binary32 -> PlanarTextureProjection
new = |origin, u_vector, v_vector|
    { origin, u_vector, v_vector }

## Creates the component for a projection onto the plane defined by the
## given origin and two vectors defining the axes along which the U and V
## texture coordinates will increase. The texture coordinates will be zero
## at the origin and unity at the tip of the respective u- or v-vector.
## Adds the component to the given entity's data.
add_new : Entity.Data, Point3.Point3 Binary32, Vector3.Vector3 Binary32, Vector3.Vector3 Binary32 -> Entity.Data
add_new = |entity_data, origin, u_vector, v_vector|
    add(entity_data, new(origin, u_vector, v_vector))

## Creates the component for a projection onto the plane defined by the
## given origin and two vectors defining the axes along which the U and V
## texture coordinates will increase. The texture coordinates will be zero
## at the origin and unity at the tip of the respective u- or v-vector.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary32), Entity.Arg.Broadcasted (Vector3.Vector3 Binary32), Entity.Arg.Broadcasted (Vector3.Vector3 Binary32) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, origin, u_vector, v_vector|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin, u_vector, v_vector,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Creates the component for a projection onto the axis-aligned horizontal
## rectangle specified by the given [`RectangleMeshComp`], scaling the
## projection so that the texture will repeat the given numbers of times
## along the U and V texture coordinate directions. The U-axis will be
## aligned with the x-axis and the V-axis will be aligned with the negative
## z-axis.
for_rectangle : Comp.RectangleMesh.RectangleMesh, F32, F32 -> PlanarTextureProjection
for_rectangle = |rectangle, n_repeats_u, n_repeats_v|
    origin = (-0.5, 0.0, 0.5)
    u_vector = (rectangle.extent_x / n_repeats_u, 0.0, 0.0)
    v_vector = (0.0, 0.0, -rectangle.extent_z / n_repeats_v)
    new(origin, u_vector, v_vector)

## Creates the component for a projection onto the axis-aligned horizontal
## rectangle specified by the given [`RectangleMeshComp`], scaling the
## projection so that the texture will repeat the given numbers of times
## along the U and V texture coordinate directions. The U-axis will be
## aligned with the x-axis and the V-axis will be aligned with the negative
## z-axis.
## Adds the component to the given entity's data.
add_for_rectangle : Entity.Data, Comp.RectangleMesh.RectangleMesh, F32, F32 -> Entity.Data
add_for_rectangle = |entity_data, rectangle, n_repeats_u, n_repeats_v|
    add(entity_data, for_rectangle(rectangle, n_repeats_u, n_repeats_v))

## Creates the component for a projection onto the axis-aligned horizontal
## rectangle specified by the given [`RectangleMeshComp`], scaling the
## projection so that the texture will repeat the given numbers of times
## along the U and V texture coordinate directions. The U-axis will be
## aligned with the x-axis and the V-axis will be aligned with the negative
## z-axis.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_rectangle : Entity.MultiData, Entity.Arg.Broadcasted (Comp.RectangleMesh.RectangleMesh), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiData Str
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
add : Entity.Data, PlanarTextureProjection -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [PlanarTextureProjection] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (PlanarTextureProjection) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in PlanarTextureProjection.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, PlanarTextureProjection -> List U8
write_packet = |bytes, val|
    type_id = 2690438958355382704
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
    type_id = 2690438958355382704
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
