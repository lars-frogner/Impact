app [main!] { pf: platform "../impact_roc/platform/main.roc" }

import pf.Stdout

main! : {} => Result {} _
main! = |{}|
    Stdout.line!("Roc loves Rust")?
    Ok({})
