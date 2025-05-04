app [
    setup_scene!,
] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Entity as Entity
import pf.Scene as Scene
import pf.Stdout as Stdout
import TestScene

setup_scene! : {} => Result {} Str
setup_scene! = |_|
    _ = Stdout.line!("setup_scene! called")

    _player = Entity.create!(TestScene.player({}))?
    _ground = Entity.create!(TestScene.ground({}))?
    _ambient_light = Entity.create!(TestScene.ambient_light({}))?
    _unidirectional_light = Entity.create!(TestScene.unidirectional_light({}))?

    Scene.set_skybox!("space_skybox", 1e6)?

    Ok({})
