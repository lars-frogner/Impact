# Hash: d40d4c7625cf3007
# Generated: 2026-08-21T14:25:41.808572284
# Rust type: impact_particle::setup::BasicParticleEmission
# Type category: Component
module [
    BasicParticleEmission,
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
import core.Hashing
import core.Point3
import core.Radians
import core.UnitVector3

## Emission of particles governed by the basic particle simulation.
BasicParticleEmission : {
    material_name_hash : Hashing.Hash32,
    log2_particle_count : U32,
    spawn_position : Point3.Point3,
    spawn_direction : UnitVector3.UnitVector3,
    cos_min_polar_spawn_angle : F32,
    cos_max_polar_spawn_angle : F32,
    min_spawn_extent : F32,
    max_spawn_extent : F32,
    min_spawn_speed : F32,
    max_spawn_speed : F32,
    min_lifetime : F32,
    max_lifetime : F32,
}

new : Str, U32, Point3.Point3, UnitVector3.UnitVector3, Radians.Radians, Radians.Radians, F32, F32, F32, F32, F32, F32 -> BasicParticleEmission
new = |material_name, log2_particle_count, spawn_position, spawn_direction, min_polar_spawn_angle, max_polar_spawn_angle, min_spawn_extent, max_spawn_extent, min_spawn_speed, max_spawn_speed, min_lifetime, max_lifetime|
    material_name_hash = Hashing.hash_str_32(material_name)
    cos_min_polar_spawn_angle = Num.cos(Num.max(0, Num.min(Num.pi, Radians.radians(min_polar_spawn_angle))))
    cos_max_polar_spawn_angle = Num.cos(Num.max(Radians.radians(min_polar_spawn_angle), Num.min(Num.pi, Radians.radians(max_polar_spawn_angle))))
    {
        material_name_hash,
        log2_particle_count,
        spawn_position,
        spawn_direction,
        cos_min_polar_spawn_angle,
        cos_max_polar_spawn_angle,
        min_spawn_extent,
        max_spawn_extent: Num.max(min_spawn_extent, max_spawn_extent),
        min_spawn_speed,
        max_spawn_speed: Num.max(min_spawn_speed, max_spawn_speed),
        min_lifetime,
        max_lifetime: Num.max(min_lifetime, max_lifetime),
    }

add_new : Entity.ComponentData, Str, U32, Point3.Point3, UnitVector3.UnitVector3, Radians.Radians, Radians.Radians, F32, F32, F32, F32, F32, F32 -> Entity.ComponentData
add_new = |entity_data, material_name, log2_particle_count, spawn_position, spawn_direction, min_polar_spawn_angle, max_polar_spawn_angle, min_spawn_extent, max_spawn_extent, min_spawn_speed, max_spawn_speed, min_lifetime, max_lifetime|
    add(entity_data, new(material_name, log2_particle_count, spawn_position, spawn_direction, min_polar_spawn_angle, max_polar_spawn_angle, min_spawn_extent, max_spawn_extent, min_spawn_speed, max_spawn_speed, min_lifetime, max_lifetime))

## Adds a value of the [BasicParticleEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, BasicParticleEmission -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [BasicParticleEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (BasicParticleEmission) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in BasicParticleEmission.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, BasicParticleEmission -> List U8
write_packet = |bytes, val|
    type_id = 8789485584940170758
    size = 64
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List BasicParticleEmission -> List U8
write_multi_packet = |bytes, vals|
    type_id = 8789485584940170758
    size = 64
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

## Serializes a value of [BasicParticleEmission] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, BasicParticleEmission -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(64)
    |> Hashing.write_bytes_hash_32(value.material_name_hash)
    |> Builtin.write_bytes_u32(value.log2_particle_count)
    |> Point3.write_bytes(value.spawn_position)
    |> UnitVector3.write_bytes(value.spawn_direction)
    |> Builtin.write_bytes_f32(value.cos_min_polar_spawn_angle)
    |> Builtin.write_bytes_f32(value.cos_max_polar_spawn_angle)
    |> Builtin.write_bytes_f32(value.min_spawn_extent)
    |> Builtin.write_bytes_f32(value.max_spawn_extent)
    |> Builtin.write_bytes_f32(value.min_spawn_speed)
    |> Builtin.write_bytes_f32(value.max_spawn_speed)
    |> Builtin.write_bytes_f32(value.min_lifetime)
    |> Builtin.write_bytes_f32(value.max_lifetime)

## Deserializes a value of [BasicParticleEmission] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result BasicParticleEmission _
from_bytes = |bytes|
    Ok(
        {
            material_name_hash: bytes |> List.sublist({ start: 0, len: 4 }) |> Hashing.from_bytes_hash_32?,
            log2_particle_count: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_u32?,
            spawn_position: bytes |> List.sublist({ start: 8, len: 12 }) |> Point3.from_bytes?,
            spawn_direction: bytes |> List.sublist({ start: 20, len: 12 }) |> UnitVector3.from_bytes?,
            cos_min_polar_spawn_angle: bytes |> List.sublist({ start: 32, len: 4 }) |> Builtin.from_bytes_f32?,
            cos_max_polar_spawn_angle: bytes |> List.sublist({ start: 36, len: 4 }) |> Builtin.from_bytes_f32?,
            min_spawn_extent: bytes |> List.sublist({ start: 40, len: 4 }) |> Builtin.from_bytes_f32?,
            max_spawn_extent: bytes |> List.sublist({ start: 44, len: 4 }) |> Builtin.from_bytes_f32?,
            min_spawn_speed: bytes |> List.sublist({ start: 48, len: 4 }) |> Builtin.from_bytes_f32?,
            max_spawn_speed: bytes |> List.sublist({ start: 52, len: 4 }) |> Builtin.from_bytes_f32?,
            min_lifetime: bytes |> List.sublist({ start: 56, len: 4 }) |> Builtin.from_bytes_f32?,
            max_lifetime: bytes |> List.sublist({ start: 60, len: 4 }) |> Builtin.from_bytes_f32?,
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
