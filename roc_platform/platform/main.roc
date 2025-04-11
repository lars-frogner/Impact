platform "impact"
    requires {} { main! : {} => Result {} [Exit I32 Str]_ }
    exposes [Stdout, Impact, Core]
    packages {}
    imports []
    provides [main_for_host!]

import Stdout

main_for_host! : I32 => I32
main_for_host! = |_|
    when main!({}) is
        Ok({}) -> 0
        Err(Exit(code, msg)) ->
            if Str.is_empty(msg) then
                code
            else
                _ = Stdout.line!(msg)
                code

        Err(msg) ->
            help_msg =
                """
                Program exited with error:
                    ${Inspect.to_str(msg)}

                Tip: If you do not want to exit on this error, use `Result.map_err` to handle the error. Docs for `Result.map_err`: <https://www.roc-lang.org/builtins/Result#map_err>
                """

            _ = Stdout.line!(help_msg)
            1
