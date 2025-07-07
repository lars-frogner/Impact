app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import Scenes.Asteroid

callbacks = {
    setup_scene!: Scenes.Asteroid.setup!,
}
