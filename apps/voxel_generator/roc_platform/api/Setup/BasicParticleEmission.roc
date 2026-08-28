# Hash: bab5e51e92871313
# Generated: 2026-08-28T19:42:02.356288224
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
import Particle.ColorOverLifetime
import Particle.RateOverTime
import Particle.ScaleOverLifetime
import Particle.SpawnDistribution
import core.Builtin
import core.Hashing

## Emission of particles governed by the basic particle simulation.
BasicParticleEmission : {
    material_name_hash : Hashing.Hash32,
    log2_particle_count : U32,
    rate_over_time : Particle.RateOverTime.RateOverTime,
    cos_polar_spawn_angle : Particle.SpawnDistribution.SpawnDistribution,
    spawn_extent : Particle.SpawnDistribution.SpawnDistribution,
    spawn_distance : Particle.SpawnDistribution.SpawnDistribution,
    spawn_speed : Particle.SpawnDistribution.SpawnDistribution,
    spawn_angle : Particle.SpawnDistribution.SpawnDistribution,
    spawn_angular_velocity : Particle.SpawnDistribution.SpawnDistribution,
    lifetime : Particle.SpawnDistribution.SpawnDistribution,
    color_over_lifetime : Particle.ColorOverLifetime.ColorOverLifetime,
    scale_over_lifetime : Particle.ScaleOverLifetime.ScaleOverLifetime,
}

new : Str, U32, Particle.RateOverTime.RateOverTime, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.ColorOverLifetime.ColorOverLifetime, Particle.ScaleOverLifetime.ScaleOverLifetime -> BasicParticleEmission
new = |material_name, log2_particle_count, rate_over_time, cos_polar_spawn_angle, spawn_extent, spawn_distance, spawn_speed, spawn_angle, spawn_angular_velocity, lifetime, color_over_lifetime, scale_over_lifetime|
    material_name_hash = Hashing.hash_str_32(material_name)
    {
        material_name_hash,
        log2_particle_count,
        rate_over_time,
        cos_polar_spawn_angle,
        spawn_extent,
        spawn_distance,
        spawn_speed,
        spawn_angle,
        spawn_angular_velocity,
        lifetime,
        color_over_lifetime,
        scale_over_lifetime,
    }

add_new : Entity.ComponentData, Str, U32, Particle.RateOverTime.RateOverTime, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.SpawnDistribution.SpawnDistribution, Particle.ColorOverLifetime.ColorOverLifetime, Particle.ScaleOverLifetime.ScaleOverLifetime -> Entity.ComponentData
add_new = |entity_data, material_name, log2_particle_count, rate_over_time, cos_polar_spawn_angle, spawn_extent, spawn_distance, spawn_speed, spawn_angle, spawn_angular_velocity, lifetime, color_over_lifetime, scale_over_lifetime|
    add(entity_data, new(material_name, log2_particle_count, rate_over_time, cos_polar_spawn_angle, spawn_extent, spawn_distance, spawn_speed, spawn_angle, spawn_angular_velocity, lifetime, color_over_lifetime, scale_over_lifetime))

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
    size = 208
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
    size = 208
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
    |> List.reserve(208)
    |> Hashing.write_bytes_hash_32(value.material_name_hash)
    |> Builtin.write_bytes_u32(value.log2_particle_count)
    |> Particle.RateOverTime.write_bytes(value.rate_over_time)
    |> Particle.SpawnDistribution.write_bytes(value.cos_polar_spawn_angle)
    |> Particle.SpawnDistribution.write_bytes(value.spawn_extent)
    |> Particle.SpawnDistribution.write_bytes(value.spawn_distance)
    |> Particle.SpawnDistribution.write_bytes(value.spawn_speed)
    |> Particle.SpawnDistribution.write_bytes(value.spawn_angle)
    |> Particle.SpawnDistribution.write_bytes(value.spawn_angular_velocity)
    |> Particle.SpawnDistribution.write_bytes(value.lifetime)
    |> Particle.ColorOverLifetime.write_bytes(value.color_over_lifetime)
    |> Particle.ScaleOverLifetime.write_bytes(value.scale_over_lifetime)

## Deserializes a value of [BasicParticleEmission] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result BasicParticleEmission _
from_bytes = |bytes|
    Ok(
        {
            material_name_hash: bytes |> List.sublist({ start: 0, len: 4 }) |> Hashing.from_bytes_hash_32?,
            log2_particle_count: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_u32?,
            rate_over_time: bytes |> List.sublist({ start: 8, len: 32 }) |> Particle.RateOverTime.from_bytes?,
            cos_polar_spawn_angle: bytes |> List.sublist({ start: 40, len: 8 }) |> Particle.SpawnDistribution.from_bytes?,
            spawn_extent: bytes |> List.sublist({ start: 48, len: 8 }) |> Particle.SpawnDistribution.from_bytes?,
            spawn_distance: bytes |> List.sublist({ start: 56, len: 8 }) |> Particle.SpawnDistribution.from_bytes?,
            spawn_speed: bytes |> List.sublist({ start: 64, len: 8 }) |> Particle.SpawnDistribution.from_bytes?,
            spawn_angle: bytes |> List.sublist({ start: 72, len: 8 }) |> Particle.SpawnDistribution.from_bytes?,
            spawn_angular_velocity: bytes |> List.sublist({ start: 80, len: 8 }) |> Particle.SpawnDistribution.from_bytes?,
            lifetime: bytes |> List.sublist({ start: 88, len: 8 }) |> Particle.SpawnDistribution.from_bytes?,
            color_over_lifetime: bytes |> List.sublist({ start: 96, len: 80 }) |> Particle.ColorOverLifetime.from_bytes?,
            scale_over_lifetime: bytes |> List.sublist({ start: 176, len: 32 }) |> Particle.ScaleOverLifetime.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 208 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
