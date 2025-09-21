# Hash: af23e392887f8eadfc9255111d0167de2d3a7013fa63d3fb60ff48146f0f430d
# Generated: 2025-08-01T06:51:20+00:00
# Rust type: impact_model::InstanceFeatureID
# Type category: POD
# Commit: 5cd592d6
module [
    InstanceFeatureID,
    write_bytes,
    from_bytes,
]

import Model.InstanceFeatureTypeID
import core.Builtin

## Identifier for an instance feature value.
InstanceFeatureID : {
    feature_type_id : Model.InstanceFeatureTypeID.InstanceFeatureTypeID,
    idx : U64,
}

## Serializes a value of [InstanceFeatureID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InstanceFeatureID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Model.InstanceFeatureTypeID.write_bytes(value.feature_type_id)
    |> Builtin.write_bytes_u64(value.idx)

## Deserializes a value of [InstanceFeatureID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InstanceFeatureID _
from_bytes = |bytes|
    Ok(
        {
            feature_type_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Model.InstanceFeatureTypeID.from_bytes?,
            idx: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_u64?,
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
