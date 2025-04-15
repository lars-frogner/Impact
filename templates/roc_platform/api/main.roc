platform "impact"
    requires {} {
        app_config_path! : {} => Str,
        setup_scene! : {} => Result {} Str,
    }
    exposes [Stdout, Entity]
    packages {}
    imports []
    provides [
        app_config_path_for_host!,
        setup_scene_for_host!,
    ]

app_config_path_for_host! : I32 => Str
app_config_path_for_host! = |_|
    app_config_path!({})

setup_scene_for_host! : I32 => Result {} Str
setup_scene_for_host! = |_|
    setup_scene!({})
