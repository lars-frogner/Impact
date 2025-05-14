# Hash: d8e6d9fcc70327e65bc89f5df6eabd286635d57476ed9766d01df06249bd9962
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::ui::command::UICommand
# Type category: Inline
# Commit: d505d37
module [
    UICommand,
    write_bytes,
    from_bytes,
]

import Command.ToInteractionMode

UICommand : [
    SetInteractionMode Command.ToInteractionMode.ToInteractionMode,
]

## Serializes a value of [UICommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UICommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetInteractionMode(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> Command.ToInteractionMode.write_bytes(val)

## Deserializes a value of [UICommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UICommand _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetInteractionMode(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToInteractionMode.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 2, 0)?
    Ok({})

test_roundtrip_for_variant : U8, U64, U64 -> Result {} _
test_roundtrip_for_variant = |discriminant, variant_size, padding_size|
    bytes = 
        List.range({ start: At discriminant, end: Length variant_size })
        |> List.concat(List.repeat(0, padding_size))
        |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
