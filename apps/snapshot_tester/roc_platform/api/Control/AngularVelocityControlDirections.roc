# Hash: c562d9bd18cc5d75
# Generated: 2026-01-14T20:29:22.271159059
# Rust type: impact_controller::orientation::AngularVelocityControlDirections
# Type category: POD
module [
    AngularVelocityControlDirections,
    empty,
    all,
    horizontal,
    vertical,
    intersects,
    contains,
    union,
    intersection,
    difference,
    write_bytes,
    from_bytes,
]

import core.Builtin

## Directions in which angular velocity can be controlled.
AngularVelocityControlDirections := U8 implements [Eq]

empty = @AngularVelocityControlDirections(0)

all = @AngularVelocityControlDirections(3)

horizontal = @AngularVelocityControlDirections(Num.shift_left_by(1, 0))

vertical = @AngularVelocityControlDirections(Num.shift_left_by(1, 1))

## Returns the raw bitflags as an unsigned integer.
bits = |@AngularVelocityControlDirections(flags)|
    flags

## Whether any set bits in the second flags value are also set in the first
## flags value.
intersects = |@AngularVelocityControlDirections(a), @AngularVelocityControlDirections(b)|
    Num.bitwise_and(a, b) != 0

## Whether all set bits in the second flags value are also set in the first
## flags value.
contains = |@AngularVelocityControlDirections(a), @AngularVelocityControlDirections(b)|
    Num.bitwise_and(a, b) == b

## The bitwise or (|) of the bits in two flags values.
union = |@AngularVelocityControlDirections(a), @AngularVelocityControlDirections(b)|
    @AngularVelocityControlDirections(Num.bitwise_or(a, b))

## The bitwise and (&) of the bits in two flags values.
intersection = |@AngularVelocityControlDirections(a), @AngularVelocityControlDirections(b)|
    @AngularVelocityControlDirections(Num.bitwise_and(a, b))

## The intersection of the first flags value with the complement of the second
## flags value (&!).
difference = |@AngularVelocityControlDirections(a), @AngularVelocityControlDirections(b)|
    @AngularVelocityControlDirections(Num.bitwise_and(a, Num.bitwise_not(b)))

## Serializes a value of [AngularVelocityControlDirections] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AngularVelocityControlDirections -> List U8
write_bytes = |bytes, value|
    Builtin.write_bytes_u8(bytes, bits(value))

## Deserializes a value of [AngularVelocityControlDirections] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AngularVelocityControlDirections _
from_bytes = |bytes|
    Builtin.from_bytes_u8(bytes) |> Result.map_ok(@AngularVelocityControlDirections)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 1 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
