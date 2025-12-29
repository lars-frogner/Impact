# Hash: c16f0421009f9148
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact_physics::medium::UniformMedium
# Type category: Inline
module [
    UniformMedium,
    sea_level_air_mass_density,
    water_mass_density,
    new,
    vacuum,
    still_air,
    moving_air,
    still_water,
    moving_water,
    write_bytes,
    from_bytes,
]

import core.Builtin
import core.Vector3

## A physical medium with the same properties and state everywhere.
UniformMedium : {
    ## The mass density of the medium.
    mass_density : F32,
    ## The velocity of the medium.
    velocity : Vector3.Vector3,
}

## Earth air mass density at sea level and room temperature [kg/m^3].
sea_level_air_mass_density : F32
sea_level_air_mass_density = 1.2

## Water mass density [kg/m^3].
water_mass_density : F32
water_mass_density = 1e3

## Creates a new uniform medium with the given mass density and velocity.
new : F32, Vector3.Vector3 -> UniformMedium
new = |mass_density, velocity|
    { mass_density, velocity }

## Creates a new vacuum medium (zero mass density and velocity).
vacuum : {} -> UniformMedium
vacuum = |{}|
    new(0.0, Vector3.zero)

## Creates a new medium of Earth air at sea level and room temperature with
## no wind.
still_air : {} -> UniformMedium
still_air = |{}|
    moving_air(Vector3.zero)

## Creates a new medium of Earth air at sea level and room temperature with
## the given wind velocity.
moving_air : Vector3.Vector3 -> UniformMedium
moving_air = |velocity|
    new(sea_level_air_mass_density, velocity)

## Creates a new medium of water with no flow.
still_water : {} -> UniformMedium
still_water = |{}|
    moving_water(Vector3.zero)

## Creates a new medium of water with the given flow velocity.
moving_water : Vector3.Vector3 -> UniformMedium
moving_water = |velocity|
    new(water_mass_density, velocity)

## Serializes a value of [UniformMedium] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UniformMedium -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f32(value.mass_density)
    |> Vector3.write_bytes(value.velocity)

## Deserializes a value of [UniformMedium] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UniformMedium _
from_bytes = |bytes|
    Ok(
        {
            mass_density: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            velocity: bytes |> List.sublist({ start: 4, len: 12 }) |> Vector3.from_bytes?,
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
