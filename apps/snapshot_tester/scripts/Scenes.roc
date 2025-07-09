module [
    setup_test_scene!,
]

import Entities
import pf.Entity

setup_test_scene! = |scene|
    when scene is
        AmbientLight -> setup_ambient_light_test!({})
        OmnidirectionalLight -> setup_omnidirectional_light_test!({})
        UnidirectionalLight -> setup_unidirectional_light_test!({})
        ShadowableOmnidirectionalLight -> setup_shadowable_omnidirectional_light_test!({})
        ShadowableUnidirectionalLight -> setup_shadowable_unidirectional_light_test!({})
        AmbientOcclusion -> setup_ambient_occlusion_test!({})
        Bloom -> setup_bloom_test!({})
        ShadowCubeMapping -> setup_shadow_cube_mapping_test!({})
        SoftShadowCubeMapping -> setup_soft_shadow_cube_mapping_test!({})
        CascadedShadowMapping -> setup_cascaded_shadow_mapping_test!({})
        SoftCascadedShadowMapping -> setup_soft_cascaded_shadow_mapping_test!({})
        # Omnidirectional light test scene works well for checking tone mapping
        ACESToneMapping -> setup_omnidirectional_light_test!({})
        KhronosPBRNeutralToneMapping -> setup_omnidirectional_light_test!({})

setup_ambient_light_test! = |_|
    setup_model_grid!({})?
    _ = Entity.create!(Entities.ambient_light)?
    Ok({})

setup_omnidirectional_light_test! = |_|
    setup_model_grid!({})?
    _ = Entity.create!(Entities.omnidirectional_light)?
    Ok({})

setup_unidirectional_light_test! = |_|
    setup_model_grid!({})?
    _ = Entity.create!(Entities.unidirectional_light)?
    Ok({})

setup_shadowable_omnidirectional_light_test! = |_|
    setup_model_grid!({})?
    _ = Entity.create!(Entities.shadowable_omnidirectional_light)?
    Ok({})

setup_shadowable_unidirectional_light_test! = |_|
    setup_model_grid!({})?
    _ = Entity.create!(Entities.shadowable_unidirectional_light)?
    Ok({})

setup_ambient_occlusion_test! = |_|
    _ = Entity.create!(Entities.tilted_camera)?
    _ = Entity.create!(Entities.ambient_occlusion_ground)?
    _ = Entity.create!(Entities.ambient_occlusion_box)?
    _ = Entity.create!(Entities.ambient_occlusion_sphere)?
    _ = Entity.create!(Entities.ambient_light)?
    Ok({})

setup_bloom_test! = |_|
    _ = Entity.create!(Entities.camera)?
    _ = Entity.create!(Entities.emissive_square)?
    _ = Entity.create!(Entities.obscuring_square)?
    Ok({})

setup_shadow_cube_mapping_test! = |_|
    setup_shadow_cube_mapping_models!({})?
    _ = Entity.create!(Entities.shadow_cube_mapping_light)?
    Ok({})

setup_soft_shadow_cube_mapping_test! = |_|
    setup_shadow_cube_mapping_models!({})?
    _ = Entity.create!(Entities.shadow_cube_mapping_soft_light)?
    Ok({})

setup_cascaded_shadow_mapping_test! = |_|
    setup_cascaded_shadow_mapping_models!({})?
    _ = Entity.create!(Entities.cascaded_shadow_mapping_light)?
    Ok({})

setup_soft_cascaded_shadow_mapping_test! = |_|
    setup_cascaded_shadow_mapping_models!({})?
    _ = Entity.create!(Entities.cascaded_shadow_mapping_soft_light)?
    Ok({})

setup_model_grid! = |_|
    _ = Entity.create!(Entities.camera)?
    _ = Entity.create!(Entities.diffuse_box)?
    _ = Entity.create!(Entities.plastic_box)?
    _ = Entity.create!(Entities.metallic_box)?
    _ = Entity.create!(Entities.diffuse_sphere)?
    _ = Entity.create!(Entities.plastic_sphere)?
    _ = Entity.create!(Entities.metallic_sphere)?
    Ok({})

setup_shadow_cube_mapping_models! = |_|
    _ = Entity.create!(Entities.tilted_camera)?
    _ = Entity.create!(Entities.shadow_cube_mapping_ground)?
    _ = Entity.create!(Entities.shadow_cube_mapping_sphere)?
    _ = Entity.create!(Entities.shadow_cube_mapping_cylinder)?
    _ = Entity.create!(Entities.shadow_cube_mapping_box)?
    Ok({})

setup_cascaded_shadow_mapping_models! = |_|
    _ = Entity.create!(Entities.tilted_camera)?
    _ = Entity.create!(Entities.cascaded_shadow_mapping_ground)?
    _ = Entity.create!(Entities.cascaded_shadow_mapping_sphere)?
    _ = Entity.create!(Entities.cascaded_shadow_mapping_cylinder)?
    _ = Entity.create!(Entities.cascaded_shadow_mapping_box)?
    Ok({})
