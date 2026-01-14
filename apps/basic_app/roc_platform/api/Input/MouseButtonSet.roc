# Hash: 212d717a4164ecc5
# Generated: 2026-01-14T20:28:34.682308962
# Rust type: impact::input::mouse::MouseButtonSet
# Type category: POD
module [
    MouseButtonSet,
    empty,
    all,
    left,
    right,
    middle,
    intersects,
    contains,
    union,
    intersection,
    difference,
    write_bytes,
    from_bytes,
]

import core.Builtin

## A set of mouse buttons.
MouseButtonSet := U8 implements [Eq]

empty = @MouseButtonSet(0)

all = @MouseButtonSet(7)

left = @MouseButtonSet(Num.shift_left_by(1, 0))

right = @MouseButtonSet(Num.shift_left_by(1, 1))

middle = @MouseButtonSet(Num.shift_left_by(1, 2))

## Returns the raw bitflags as an unsigned integer.
bits = |@MouseButtonSet(flags)|
    flags

## Whether any set bits in the second flags value are also set in the first
## flags value.
intersects = |@MouseButtonSet(a), @MouseButtonSet(b)|
    Num.bitwise_and(a, b) != 0

## Whether all set bits in the second flags value are also set in the first
## flags value.
contains = |@MouseButtonSet(a), @MouseButtonSet(b)|
    Num.bitwise_and(a, b) == b

## The bitwise or (|) of the bits in two flags values.
union = |@MouseButtonSet(a), @MouseButtonSet(b)|
    @MouseButtonSet(Num.bitwise_or(a, b))

## The bitwise and (&) of the bits in two flags values.
intersection = |@MouseButtonSet(a), @MouseButtonSet(b)|
    @MouseButtonSet(Num.bitwise_and(a, b))

## The intersection of the first flags value with the complement of the second
## flags value (&!).
difference = |@MouseButtonSet(a), @MouseButtonSet(b)|
    @MouseButtonSet(Num.bitwise_and(a, Num.bitwise_not(b)))

## Serializes a value of [MouseButtonSet] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MouseButtonSet -> List U8
write_bytes = |bytes, value|
    Builtin.write_bytes_u8(bytes, bits(value))

## Deserializes a value of [MouseButtonSet] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MouseButtonSet _
from_bytes = |bytes|
    Builtin.from_bytes_u8(bytes) |> Result.map_ok(@MouseButtonSet)

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
