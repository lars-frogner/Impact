module [set_skybox!]

import Platform

## Sets the skybox for the current scene to use the given cubemap texture and
## maximum luminance.
set_skybox! : Str, F32 => Result {} Str
set_skybox! = |cubemap_texture_name, max_luminance|
    Platform.set_skybox!(cubemap_texture_name, max_luminance)
