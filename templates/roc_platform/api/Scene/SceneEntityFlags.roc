module [
    SceneEntityFlags,
    empty,
    is_disabled,
    casts_no_shadows,
    intersects,
    contains,
    union,
    intersection,
    difference,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## Bitflags encoding a set of binary states or properties for an entity in
## a scene.
SceneEntityFlags := U8 implements [Eq]

empty = @SceneEntityFlags(0)

## The entity should not affect the scene in any way.
is_disabled = @SceneEntityFlags(Num.shift_left_by(1, 0))

## The entity should not participate in shadow maps.
casts_no_shadows = @SceneEntityFlags(Num.shift_left_by(1, 1))

## Whether any set bits in the second flags value are also set in the first
## flags value.
intersects = |@SceneEntityFlags(a), @SceneEntityFlags(b)|
    Num.bitwise_and(a, b) != 0

## Whether all set bits in the second flags value are also set in the first
## flags value.
contains = |@SceneEntityFlags(a), @SceneEntityFlags(b)|
    Num.bitwise_and(a, b) == b

## The bitwise or (|) of the bits in two flags values.
union = |@SceneEntityFlags(a), @SceneEntityFlags(b)|
    @SceneEntityFlags(Num.bitwise_or(a, b))

## The bitwise and (&) of the bits in two flags values.
intersection = |@SceneEntityFlags(a), @SceneEntityFlags(b)|
    @SceneEntityFlags(Num.bitwise_and(a, b))

## The intersection of the first flags value with the complement of the second
## flags value (&!).
difference = |@SceneEntityFlags(a), @SceneEntityFlags(b)|
    @SceneEntityFlags(Num.bitwise_and(a, Num.bitwise_not(b)))

## Adds a value of the [SceneEntityFlags] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneEntityFlags -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SceneEntityFlags] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted SceneEntityFlags -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SceneEntityFlags.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, SceneEntityFlags -> List U8
write_packet = |bytes, val|
    type_id = 6120273427405117318
    size = 1
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SceneEntityFlags -> List U8
write_multi_packet = |bytes, vals|
    type_id = 6120273427405117318
    size = 1
    alignment = 1
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

write_bytes = |bytes, @SceneEntityFlags(value)|
    Builtin.write_bytes_u8(bytes, value)

from_bytes = |bytes|
    Builtin.from_bytes_u8(bytes) |> Result.map_ok(@SceneEntityFlags)
