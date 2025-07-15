# Hash: b67d127296d9eb4a6fe86ed777eca5d3d6a6eb328dfb5274578ef84667610a43
# Generated: 2025-07-15T11:05:49+00:00
# Rust type: impact_physics::force::detailed_drag::DetailedDragForce
# Type category: POD
# Commit: 189570ab (dirty)
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
    drag_coefficient : F64,
    ## The ID of the [`DragLoadMap`] encoding the shape-dependence of the drag
    ## force.
    drag_load_map : Physics.DragLoadMapID.DragLoadMapID,
    ## The scale of the body relative to the mesh the drag load map was
    ## computed from.
    scaling : F64,
}

## Serializes a value of [DetailedDragForce] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DetailedDragForce -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Builtin.write_bytes_f64(value.drag_coefficient)
    |> Physics.DragLoadMapID.write_bytes(value.drag_load_map)
    |> Builtin.write_bytes_f64(value.scaling)

## Deserializes a value of [DetailedDragForce] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DetailedDragForce _
from_bytes = |bytes|
    Ok(
        {
            drag_coefficient: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            drag_load_map: bytes |> List.sublist({ start: 8, len: 8 }) |> Physics.DragLoadMapID.from_bytes?,
            scaling: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
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
