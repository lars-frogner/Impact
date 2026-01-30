module [lookup!]

import core.Builtin
import Platform

lookup! : List U8, (List U8 -> Result a Builtin.DecodeErr) => Result a Str
lookup! = |target_bytes, decode|
    target_bytes
    |> Platform.lookup_game_target!?
    |> decode
    |> Result.map_err(|err| "Failed to decode looked up value: ${Inspect.to_str(err)}")
