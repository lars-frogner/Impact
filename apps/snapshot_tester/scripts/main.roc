app [callbacks] {
    pf: platform "../roc_platform/api/main.roc",
    core: "../../../roc_packages/core/main.roc",
}

import Scenes

callbacks = {
    setup_scene!: Scenes.setup_test_scene!,
}
