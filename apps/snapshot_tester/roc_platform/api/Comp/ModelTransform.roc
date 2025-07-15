# Hash: e0a1c6e7db2ada729c8664cc59d691bc73e19ce703b9fd6e292efc92e97692e1
# Generated: 2025-07-15T11:05:49+00:00
# Rust type: impact_geometry::reference_frame::ModelTransform
# Type category: Component
# Commit: 189570ab (dirty)
module [
    ModelTransform,
    identity,
    with_offset,
    with_scale,
    with_offset_and_scale,
    add_identity,
    add_multiple_identity,
    add_with_offset,
    add_multiple_with_offset,
    add_with_scale,
    add_multiple_with_scale,
    add_with_offset_and_scale,
    add_multiple_with_offset_and_scale,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Vector3

## The similarity transform from the local space of a model to the space of
## a parent entity using the model.
ModelTransform : {
    ## The offset applied to a model-space position before scaling to
    ## transform it to the parent entity's space.
    offset : Vector3.Vector3 Binary32,
    ## The scaling factor applied to a model-space position after the
    ## offset to transform it to the parent entity's space.
    scale : F32,
}

## Creates a transform where the parent entity's space is identical to that
## of the model.
identity : {} -> ModelTransform
identity = |{}|
    with_scale(1.0)

## Creates a transform where the parent entity's space is identical to that
## of the model.
## Adds the component to the given entity's data.
add_identity : Entity.Data -> Entity.Data
add_identity = |entity_data|
    add(entity_data, identity({}))

## Creates a transform where the parent entity's space is identical to that
## of the model.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_identity : Entity.MultiData -> Entity.MultiData
add_multiple_identity = |entity_data|
    res = add_multiple(
        entity_data,
        Same(identity({}))
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in ModelTransform.add_multiple_identity: ${Inspect.to_str(err)}"

## Creates a transform where the parent entity's space has the given offset
## from that of the model.
with_offset : Vector3.Vector3 Binary32 -> ModelTransform
with_offset = |offset|
    with_offset_and_scale(offset, 1.0)

## Creates a transform where the parent entity's space has the given offset
## from that of the model.
## Adds the component to the given entity's data.
add_with_offset : Entity.Data, Vector3.Vector3 Binary32 -> Entity.Data
add_with_offset = |entity_data, offset|
    add(entity_data, with_offset(offset))

## Creates a transform where the parent entity's space has the given offset
## from that of the model.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_with_offset : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary32) -> Result Entity.MultiData Str
add_multiple_with_offset = |entity_data, offset|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            offset,
            Entity.multi_count(entity_data),
            with_offset
        ))
    )

## Creates a transform where the parent entity's space has the given scale
## relative to that of the model.
with_scale : F32 -> ModelTransform
with_scale = |scale|
    with_offset_and_scale(Vector3.zero, scale)

## Creates a transform where the parent entity's space has the given scale
## relative to that of the model.
## Adds the component to the given entity's data.
add_with_scale : Entity.Data, F32 -> Entity.Data
add_with_scale = |entity_data, scale|
    add(entity_data, with_scale(scale))

## Creates a transform where the parent entity's space has the given scale
## relative to that of the model.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_with_scale : Entity.MultiData, Entity.Arg.Broadcasted (F32) -> Result Entity.MultiData Str
add_multiple_with_scale = |entity_data, scale|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            scale,
            Entity.multi_count(entity_data),
            with_scale
        ))
    )

## Creates a transform where the parent entity's space has the given offset
## and scale relative to that of the model.
with_offset_and_scale : Vector3.Vector3 Binary32, F32 -> ModelTransform
with_offset_and_scale = |offset, scale|
    { offset, scale }

## Creates a transform where the parent entity's space has the given offset
## and scale relative to that of the model.
## Adds the component to the given entity's data.
add_with_offset_and_scale : Entity.Data, Vector3.Vector3 Binary32, F32 -> Entity.Data
add_with_offset_and_scale = |entity_data, offset, scale|
    add(entity_data, with_offset_and_scale(offset, scale))

## Creates a transform where the parent entity's space has the given offset
## and scale relative to that of the model.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_with_offset_and_scale : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiData Str
add_multiple_with_offset_and_scale = |entity_data, offset, scale|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            offset, scale,
            Entity.multi_count(entity_data),
            with_offset_and_scale
        ))
    )

## Adds a value of the [ModelTransform] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ModelTransform -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ModelTransform] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ModelTransform) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ModelTransform.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ModelTransform -> List U8
write_packet = |bytes, val|
    type_id = 1943320543466053004
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ModelTransform -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1943320543466053004
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

## Serializes a value of [ModelTransform] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ModelTransform -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Vector3.write_bytes_32(value.offset)
    |> Builtin.write_bytes_f32(value.scale)

## Deserializes a value of [ModelTransform] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ModelTransform _
from_bytes = |bytes|
    Ok(
        {
            offset: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
            scale: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
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
