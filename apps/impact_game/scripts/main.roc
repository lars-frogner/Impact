app [
    app_config_path!,
    setup_scene!,
] {
    pf: platform "../roc_platform/api/main.roc",
    impact: "../../../roc_packages/core/main.roc",
}

import pf.Stdout as Stdout

app_config_path! : {} => Str
app_config_path! = |_|
    _ = Stdout.line!("app_config_path! called")
    "config/config.ron"

setup_scene! : {} => Result {} Str
setup_scene! = |_|
    _ = Stdout.line!("setup_scene! called")
    # Err("Scene setup failed!!!")
    Ok({})
