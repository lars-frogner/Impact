# Hash: 3a9acba8d84023c90db75596a6963f6c5b3f8be377c6af9a17fdf94adcad5766
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact_physics::force::local_force::LocalForceGenerator
# Type category: POD
# Commit: 189570ab (dirty)
module [
    LocalForceGenerator,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Setup.LocalForce

## Generator for a constant body-space force applied to a specific point on
## a dynamic rigid body.
LocalForceGenerator : {
    ## The dynamic rigid body experiencing the force.
    rigid_body_id : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The force and its point of application, all in the body's local
    ## reference frame.
    local_force : Setup.LocalForce.LocalForce,
}

## Serializes a value of [LocalForceGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LocalForceGenerator -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.LocalForce.write_bytes(value.local_force)

## Deserializes a value of [LocalForceGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LocalForceGenerator _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            local_force: bytes |> List.sublist({ start: 8, len: 48 }) |> Setup.LocalForce.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 56 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
