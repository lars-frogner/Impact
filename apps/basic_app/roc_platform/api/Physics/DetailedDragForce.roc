# Hash: c425a219daefbec814748f72bc4e519c8d0cd8d72954bd1f973ed3f0ab3697fd
# Generated: 2025-12-17T23:58:02+00:00
# Rust type: impact_physics::force::detailed_drag::DetailedDragForce
# Type category: POD
# Commit: 7d41822d (dirty)
module [
    DetailedDragForce,
    write_bytes,
    from_bytes,
]

import Physics.DragLoadMapID
import core.Builtin

## A shape-dependent drag force on a dynamic rigid body.
DetailedDragForce : {
    ## The drag coefficient of the body.
    drag_coefficient : F32,
    ## The ID of the [`DragLoadMap`] encoding the shape-dependence of the drag
    ## force.
    drag_load_map : Physics.DragLoadMapID.DragLoadMapID,
    ## The scale of the body relative to the mesh the drag load map was
    ## computed from.
    scaling : F32,
}

## Serializes a value of [DetailedDragForce] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DetailedDragForce -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32(value.drag_coefficient)
    |> Physics.DragLoadMapID.write_bytes(value.drag_load_map)
    |> Builtin.write_bytes_f32(value.scaling)

## Deserializes a value of [DetailedDragForce] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DetailedDragForce _
from_bytes = |bytes|
    Ok(
        {
            drag_coefficient: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            drag_load_map: bytes |> List.sublist({ start: 4, len: 4 }) |> Physics.DragLoadMapID.from_bytes?,
            scaling: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 12 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
