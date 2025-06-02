module [
    Command,
    execute!,
]

import Command.EngineCommand as EngineCommand exposing [EngineCommand]
import Command.UICommand as UICommand exposing [UICommand]
import Platform

Command : [UI UICommand, Engine EngineCommand]

execute! : Command => Result {} Str
execute! = |command|
    when command is
        UI(ui_command) ->
            []
            |> UICommand.write_bytes(ui_command)
            |> Platform.execute_ui_command!

        Engine(engine_command) ->
            []
            |> EngineCommand.write_bytes(engine_command)
            |> Platform.execute_engine_command!
