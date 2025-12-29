# Hash: 6de38bc59853d8f8
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_physics::force::detailed_drag::DetailedDragForceGenerator
# Type category: POD
module [
    DetailedDragForceGenerator,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Physics.DetailedDragForce
import core.Builtin

## Generator for a shape-dependent drag force on a dynamic rigid body.
DetailedDragForceGenerator : {
    ## The dynamic rigid body experiencing the drag.
    body : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The drag force on the body.
    force : Physics.DetailedDragForce.DetailedDragForce,
    padding : F32,
}

## Serializes a value of [DetailedDragForceGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DetailedDragForceGenerator -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Comp.DynamicRigidBodyID.write_bytes(value.body)
    |> Physics.DetailedDragForce.write_bytes(value.force)
    |> Builtin.write_bytes_f32(value.padding)

## Deserializes a value of [DetailedDragForceGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DetailedDragForceGenerator _
from_bytes = |bytes|
    Ok(
        {
            body: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            force: bytes |> List.sublist({ start: 8, len: 12 }) |> Physics.DetailedDragForce.from_bytes?,
            padding: bytes |> List.sublist({ start: 20, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
