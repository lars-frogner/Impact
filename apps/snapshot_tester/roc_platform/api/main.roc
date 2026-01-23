platform "impact"
    requires {} {
        callbacks : {
            setup_scene! : TestScene => Result {} Str,
        },
    }
    exposes [
        Command,
        Comp,
        Containers,
        Control,
        Entity,
        Input,
        Light,
        Mesh,
        Model,
        Physics,
        Rendering,
        Scene,
        Setup,
        Skybox,
        Stdout,
        Test,
        Texture,
        Voxel,
    ]
    packages {
        core: "../../../../roc_packages/core/main.roc",
    }
    imports []
    provides [
        setup_scene_extern!,
    ]

import Test.TestScene as TestScene exposing [TestScene]

setup_scene_extern! : List U8 => Result {} Str
setup_scene_extern! = |bytes|
    scene = TestScene.from_bytes(bytes) |> map_err_to_str?
    callbacks.setup_scene!(scene)

map_err_to_str = |result|
    result |> Result.map_err(|err| Inspect.to_str(err))
