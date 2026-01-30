# Hash: d147159306d34426
# Generated: 2026-01-30T20:58:23.953353374
# Rust type: impact_game::lookup::GameLookupTarget
# Type category: Inline
module [
    GameLookupTarget,
    lookup!,
    write_bytes,
    from_bytes,
]

import Lookup

GameLookupTarget : [
    InventoryMass,
]

lookup! : GameLookupTarget, _ => _
lookup! = |self, decode|
    [] |> write_bytes(self) |> Lookup.lookup!(decode)

## Serializes a value of [GameLookupTarget] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GameLookupTarget -> List U8
write_bytes = |bytes, value|
    when value is
        InventoryMass ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

## Deserializes a value of [GameLookupTarget] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GameLookupTarget _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(InventoryMass)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
