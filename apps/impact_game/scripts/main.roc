app [
    setup_scene!,
] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import pf.Entity as Entity
import pf.Stdout as Stdout
import Scene

setup_scene! : {} => Result {} Str
setup_scene! = |_|
    _ = Stdout.line!("setup_scene! called")

    _entity = Entity.create!(Scene.create_player({}))?

    Ok({})
