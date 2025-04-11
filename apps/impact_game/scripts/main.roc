app [main!] { pf: platform "../../../roc_platform/platform/main.roc" }

import Generated.RoundtripTest exposing [test_roundtrip!]

main! = |{}|
    test_roundtrip!({})
    Ok({})
