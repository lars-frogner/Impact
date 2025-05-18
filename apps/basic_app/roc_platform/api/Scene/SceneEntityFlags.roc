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
    write_bytes,
    from_bytes,
]

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

write_bytes = |bytes, @SceneEntityFlags(value)|
    Builtin.write_bytes_u8(bytes, value)

from_bytes = |bytes|
    Builtin.from_bytes_u8(bytes) |> Result.map_ok(@SceneEntityFlags)
