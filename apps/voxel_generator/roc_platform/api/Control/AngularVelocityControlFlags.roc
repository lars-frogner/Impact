# Hash: ac70fde438a88c0f
# Generated: 2026-01-16T07:59:07.175914812
# Rust type: impact_controller::orientation::AngularVelocityControlFlags
# Type category: POD
module [
    AngularVelocityControlFlags,
    empty,
    all,
    preserve_existing_for_horizontal,
    intersects,
    contains,
    union,
    intersection,
    difference,
    write_bytes,
    from_bytes,
]

import core.Builtin

## Flags for how to control angular velocity.
AngularVelocityControlFlags := U32 implements [Eq]

empty = @AngularVelocityControlFlags(0)

all = @AngularVelocityControlFlags(1)

preserve_existing_for_horizontal = @AngularVelocityControlFlags(Num.shift_left_by(1, 0))

## Returns the raw bitflags as an unsigned integer.
bits = |@AngularVelocityControlFlags(flags)|
    flags

## Whether any set bits in the second flags value are also set in the first
## flags value.
intersects = |@AngularVelocityControlFlags(a), @AngularVelocityControlFlags(b)|
    Num.bitwise_and(a, b) != 0

## Whether all set bits in the second flags value are also set in the first
## flags value.
contains = |@AngularVelocityControlFlags(a), @AngularVelocityControlFlags(b)|
    Num.bitwise_and(a, b) == b

## The bitwise or (|) of the bits in two flags values.
union = |@AngularVelocityControlFlags(a), @AngularVelocityControlFlags(b)|
    @AngularVelocityControlFlags(Num.bitwise_or(a, b))

## The bitwise and (&) of the bits in two flags values.
intersection = |@AngularVelocityControlFlags(a), @AngularVelocityControlFlags(b)|
    @AngularVelocityControlFlags(Num.bitwise_and(a, b))

## The intersection of the first flags value with the complement of the second
## flags value (&!).
difference = |@AngularVelocityControlFlags(a), @AngularVelocityControlFlags(b)|
    @AngularVelocityControlFlags(Num.bitwise_and(a, Num.bitwise_not(b)))

## Serializes a value of [AngularVelocityControlFlags] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AngularVelocityControlFlags -> List U8
write_bytes = |bytes, value|
    Builtin.write_bytes_u32(bytes, bits(value))

## Deserializes a value of [AngularVelocityControlFlags] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AngularVelocityControlFlags _
from_bytes = |bytes|
    Builtin.from_bytes_u32(bytes) |> Result.map_ok(@AngularVelocityControlFlags)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 4 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
