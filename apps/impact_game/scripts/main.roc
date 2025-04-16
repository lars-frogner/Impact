app [
    setup_scene!,
] {
    pf: platform "../roc_platform/api/main.roc",
    impact: "../../../roc_packages/core/main.roc",
}

import pf.Stdout as Stdout

setup_scene! : {} => Result {} Str
setup_scene! = |_|
    _ = Stdout.line!("setup_scene! called")
    # Err("Scene setup failed!!!")
    Ok({})
