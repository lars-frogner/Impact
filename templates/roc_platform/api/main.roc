platform "impact"
    requires {} {
        setup_scene! : {} => Result {} Str,
    }
    exposes [Stdout, Entity, Scene]
    packages {}
    imports []
    provides [
        setup_scene_for_host!,
    ]

setup_scene_for_host! : I32 => Result {} Str
setup_scene_for_host! = |_|
    setup_scene!({})
