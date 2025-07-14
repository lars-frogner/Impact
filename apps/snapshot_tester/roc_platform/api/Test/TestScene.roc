# Hash: c48519b25704b7dc3c8329807ceccb919fa4c626c4eda7e68ccc0a52a0fcd2a2
# Generated: 2025-07-13T20:18:37+00:00
# Rust type: snapshot_tester::testing::TestScene
# Type category: Inline
# Commit: b1b4dfd8 (dirty)
module [
    TestScene,
    write_bytes,
    from_bytes,
]

TestScene : [
    AmbientLight,
    OmnidirectionalLight,
    UnidirectionalLight,
    ShadowableOmnidirectionalLight,
    ShadowableUnidirectionalLight,
    ShadowCubeMapping,
    SoftShadowCubeMapping,
    CascadedShadowMapping,
    SoftCascadedShadowMapping,
    AmbientOcclusion,
    Bloom,
    ACESToneMapping,
    KhronosPBRNeutralToneMapping,
]

## Serializes a value of [TestScene] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, TestScene -> List U8
write_bytes = |bytes, value|
    when value is
        AmbientLight ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        OmnidirectionalLight ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        UnidirectionalLight ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        ShadowableOmnidirectionalLight ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        ShadowableUnidirectionalLight ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        ShadowCubeMapping ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        SoftShadowCubeMapping ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        CascadedShadowMapping ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        SoftCascadedShadowMapping ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        AmbientOcclusion ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

        Bloom ->
            bytes
            |> List.reserve(1)
            |> List.append(10)

        ACESToneMapping ->
            bytes
            |> List.reserve(1)
            |> List.append(11)

        KhronosPBRNeutralToneMapping ->
            bytes
            |> List.reserve(1)
            |> List.append(12)

## Deserializes a value of [TestScene] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result TestScene _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(AmbientLight)
            [1, ..] -> Ok(OmnidirectionalLight)
            [2, ..] -> Ok(UnidirectionalLight)
            [3, ..] -> Ok(ShadowableOmnidirectionalLight)
            [4, ..] -> Ok(ShadowableUnidirectionalLight)
            [5, ..] -> Ok(ShadowCubeMapping)
            [6, ..] -> Ok(SoftShadowCubeMapping)
            [7, ..] -> Ok(CascadedShadowMapping)
            [8, ..] -> Ok(SoftCascadedShadowMapping)
            [9, ..] -> Ok(AmbientOcclusion)
            [10, ..] -> Ok(Bloom)
            [11, ..] -> Ok(ACESToneMapping)
            [12, ..] -> Ok(KhronosPBRNeutralToneMapping)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
