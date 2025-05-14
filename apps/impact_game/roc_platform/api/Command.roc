module [
    execute!,
]

import Command.EngineCommand as EngineCommand exposing [EngineCommand]
import Platform

execute! : EngineCommand => Result {} Str
execute! = |command|
    []
    |> EngineCommand.write_bytes(command)
    |> Platform.execute_engine_command!
