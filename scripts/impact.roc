app [main!] { pf: platform "../impact_roc/platform/main.roc" }

import pf.Impact

main! : {} => Result {} [Exit I32 Str]
main! = |_|
    Impact.run!({}) |> Result.map_err(|msg| Exit 1 msg)
