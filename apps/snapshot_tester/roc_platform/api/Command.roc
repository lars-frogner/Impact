module [
    Command,
    execute!,
]

import Command.EngineCommand as EngineCommand exposing [EngineCommand]
import Platform

Command : [Engine EngineCommand]

execute! : Command => Result {} Str
execute! = |command|
    when command is
        Engine(engine_command) ->
            []
            |> EngineCommand.write_bytes(engine_command)
            |> Platform.execute_engine_command!
