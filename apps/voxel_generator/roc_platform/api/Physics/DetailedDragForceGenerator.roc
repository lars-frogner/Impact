# Hash: afeb42141aba079b88aed8d0c41cb3f148c9ff47354313ea04799fb707e4e10c
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_physics::force::detailed_drag::DetailedDragForceGenerator
# Type category: POD
# Commit: 397d36d3 (dirty)
module [
    DetailedDragForceGenerator,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Physics.DetailedDragForce

## Generator for a shape-dependent drag force on a dynamic rigid body.
DetailedDragForceGenerator : {
    ## The dynamic rigid body experiencing the drag.
    body : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The drag force on the body.
    force : Physics.DetailedDragForce.DetailedDragForce,
}

## Serializes a value of [DetailedDragForceGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DetailedDragForceGenerator -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Comp.DynamicRigidBodyID.write_bytes(value.body)
    |> Physics.DetailedDragForce.write_bytes(value.force)

## Deserializes a value of [DetailedDragForceGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DetailedDragForceGenerator _
from_bytes = |bytes|
    Ok(
        {
            body: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            force: bytes |> List.sublist({ start: 8, len: 24 }) |> Physics.DetailedDragForce.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 32 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
