# Hash: b59c2b03277f349d7d2dbad1d55940bd0cb203c2139a8659e54cba6811c954f0
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::MaterialHandle
# Type category: POD
# Commit: d505d37
module [
    MaterialHandle,
    write_bytes,
    from_bytes,
]

import Material.MaterialID
import Material.MaterialPropertyTextureGroupID
import Model.InstanceFeatureID

## A handle for a material, containing the IDs for the pieces of data holding
## information about the material.
MaterialHandle : {
    ## The ID of the material's [`MaterialSpecification`].
    material_id : Material.MaterialID.MaterialID,
    ## The ID of the entry for the material's per-instance material properties
    ## in the [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage)
    ## (may be N/A).
    material_property_feature_id : Model.InstanceFeatureID.InstanceFeatureID,
    ## The ID of the material's [`MaterialPropertyTextureGroup`] (may represent
    ## an empty group).
    material_property_texture_group_id : Material.MaterialPropertyTextureGroupID.MaterialPropertyTextureGroupID,
}

## Serializes a value of [MaterialHandle] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MaterialHandle -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Material.MaterialID.write_bytes(value.material_id)
    |> Model.InstanceFeatureID.write_bytes(value.material_property_feature_id)
    |> Material.MaterialPropertyTextureGroupID.write_bytes(value.material_property_texture_group_id)

## Deserializes a value of [MaterialHandle] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MaterialHandle _
from_bytes = |bytes|
    Ok(
        {
            material_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Material.MaterialID.from_bytes?,
            material_property_feature_id: bytes |> List.sublist({ start: 8, len: 16 }) |> Model.InstanceFeatureID.from_bytes?,
            material_property_texture_group_id: bytes |> List.sublist({ start: 24, len: 8 }) |> Material.MaterialPropertyTextureGroupID.from_bytes?,
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
