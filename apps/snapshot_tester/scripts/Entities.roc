module [
    camera,
    tilted_camera,
    ambient_light,
    omnidirectional_light,
    unidirectional_light,
    shadowable_omnidirectional_light,
    shadowable_unidirectional_light,
    diffuse_sphere,
    plastic_sphere,
    metallic_sphere,
    diffuse_box,
    plastic_box,
    metallic_box,
    emissive_square,
    obscuring_square,
    ambient_occlusion_ground,
    ambient_occlusion_box,
    ambient_occlusion_sphere,
    shadow_cube_mapping_light,
    shadow_cube_mapping_soft_light,
    shadow_cube_mapping_ground,
    shadow_cube_mapping_sphere,
    shadow_cube_mapping_cylinder,
    shadow_cube_mapping_box,
    cascaded_shadow_mapping_light,
    cascaded_shadow_mapping_soft_light,
    cascaded_shadow_mapping_ground,
    cascaded_shadow_mapping_sphere,
    cascaded_shadow_mapping_cylinder,
    cascaded_shadow_mapping_box,
]

import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Entity
import pf.Setup.BoxMesh
import pf.Setup.CylinderMesh
import pf.Setup.RectangleMesh
import pf.Setup.SphereMesh
import pf.Setup.PerspectiveCamera
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Setup.UniformMetalness
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Comp.AmbientEmission
import pf.Comp.OmnidirectionalEmission
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Comp.UnidirectionalEmission

# **** Camera ****

camera =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0, 0, 0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(50), 0.01, 1000)

tilted_camera =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0, 0, 0),
        UnitQuaternion.mul(
            UnitQuaternion.from_axis_angle(UnitVector3.x_axis, 0.5),
            UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
        ),
    )
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(50), 0.01, 1000)

# **** Lights ****

ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(3e3))

omnidirectional_light =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 0))
    |> Comp.OmnidirectionalEmission.add_new(
        Vector3.same(1e4),
        0.4,
    )

unidirectional_light =
    Entity.new_component_data
    |> Comp.UnidirectionalEmission.add_new(
        Vector3.same(3e3),
        UnitVector3.from((0.0, 0.0, 1.0)),
        10.0,
    )

shadowable_omnidirectional_light =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, 0, 0))
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(1e4),
        0.4,
    )

shadowable_unidirectional_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(3e3),
        UnitVector3.from((0.0, 0.0, 1.0)),
        10.0,
    )

# **** Mesh material grid ****

dist = 2.8

voffset = 0.1
hspacing = 0.05
vspacing = 0.05

box_scale = 0.75
box_height = -0.5

sphere_rings = 15
sphere_height = 0.5

diffuse_box =
    Entity.new_component_data
    |> Setup.BoxMesh.add_unit_cube
    |> Comp.ModelTransform.add_with_scale(box_scale)
    |> Comp.ReferenceFrame.add_unoriented((1 + hspacing, box_height - vspacing + voffset, dist))
    |> add_diffuse

plastic_box =
    Entity.new_component_data
    |> Setup.BoxMesh.add_unit_cube
    |> Comp.ModelTransform.add_with_scale(box_scale)
    |> Comp.ReferenceFrame.add_unoriented((0, box_height - vspacing + voffset, dist))
    |> add_plastic

metallic_box =
    Entity.new_component_data
    |> Setup.BoxMesh.add_unit_cube
    |> Comp.ModelTransform.add_with_scale(box_scale)
    |> Comp.ReferenceFrame.add_unoriented((-1 - hspacing, box_height - vspacing + voffset, dist))
    |> add_metallic

diffuse_sphere =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(sphere_rings)
    |> Comp.ReferenceFrame.add_unoriented((1 + hspacing, sphere_height + vspacing + voffset, dist))
    |> add_diffuse

plastic_sphere =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(sphere_rings)
    |> Comp.ReferenceFrame.add_unoriented((0, sphere_height + vspacing + voffset, dist))
    |> add_plastic

metallic_sphere =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(sphere_rings)
    |> Comp.ReferenceFrame.add_unoriented((-1 - hspacing, sphere_height + vspacing + voffset, dist))
    |> add_metallic

# **** Materials ****

add_diffuse = |entity|
    entity
    |> Setup.UniformColor.add((0.4, 0.8, 0.3))
    |> Setup.UniformRoughness.add(0.7)

add_plastic = |entity|
    entity
    |> Setup.UniformColor.add((0.3, 0.4, 0.8))
    |> Setup.UniformSpecularReflectance.add(0.05)
    |> Setup.UniformRoughness.add(0.3)

add_metallic = |entity|
    entity
    |> Setup.UniformColor.add_gold
    |> Setup.UniformSpecularReflectance.add_metal
    |> Setup.UniformMetalness.add_metal
    |> Setup.UniformRoughness.add(0.5)

# **** Bloom ****

emissive_square =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ReferenceFrame.add_new(
        (0, 0, 1.5),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
    )
    |> Setup.UniformColor.add((1, 1, 1))
    |> Setup.UniformEmissiveLuminance.add(1e6)

obscuring_square =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(0.5)
    |> Comp.ReferenceFrame.add_new(
        (0, 0, 1.4),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, (-Num.pi) / 2),
    )
    |> Setup.UniformColor.add((0, 0, 0))

# **** Ambient occlusion ****

ao_ground_height = -2.0
ao_box_scale = 1.0
ao_box_hshift = 0.6
ao_sphere_scale = 1.2

ambient_occlusion_ground =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(10.0)
    |> Comp.ReferenceFrame.add_unoriented((0, ao_ground_height, 5))
    |> add_metallic

ambient_occlusion_box =
    Entity.new_component_data
    |> Setup.BoxMesh.add_unit_cube
    |> Comp.ModelTransform.add_with_scale(ao_box_scale)
    |> Comp.ReferenceFrame.add_new(
        (ao_box_hshift, ao_ground_height + ao_box_scale / 2, 3),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, -0.1),
    )
    |> add_diffuse

ambient_occlusion_sphere =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(sphere_rings)
    |> Comp.ModelTransform.add_with_scale(ao_sphere_scale)
    |> Comp.ReferenceFrame.add_unoriented(
        (ao_box_hshift - (ao_box_scale + ao_sphere_scale) / 2, ao_ground_height + ao_sphere_scale / 2, 2.8),
    )
    |> add_plastic

# **** Shadow cube mapping ****

scm_dist = 4.5
scm_ground_height = -2.0
scm_sphere_scale = 0.8
scm_box_scale = 0.6

shadow_cube_mapping_light =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, scm_ground_height + 1.8, scm_dist))
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(1e4),
        0.0,
    )

shadow_cube_mapping_soft_light =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((0, scm_ground_height + 1.8, scm_dist))
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(1e4),
        0.2,
    )

shadow_cube_mapping_ground =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(2 * scm_dist)
    |> Comp.ReferenceFrame.add_unoriented((0, scm_ground_height, scm_dist))
    |> add_diffuse

shadow_cube_mapping_sphere =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(sphere_rings)
    |> Comp.ModelTransform.add_with_scale(scm_sphere_scale)
    |> Comp.ReferenceFrame.add_unoriented(
        (-0.8, scm_ground_height + scm_sphere_scale / 2, scm_dist - 1.5),
    )
    |> add_plastic

shadow_cube_mapping_cylinder =
    Entity.new_component_data
    |> Setup.CylinderMesh.add_new(1.5, 0.2, 15)
    |> Comp.ReferenceFrame.add_unoriented((-0.4, scm_ground_height, scm_dist + 0.6))
    |> add_plastic

shadow_cube_mapping_box =
    Entity.new_component_data
    |> Setup.BoxMesh.add_unit_cube
    |> Comp.ModelTransform.add_with_scale(csm_box_scale)
    |> Comp.ReferenceFrame.add_unoriented(
        (0.8, scm_ground_height + scm_box_scale / 2, scm_dist - 0.6),
    )
    |> add_plastic

# **** Cascaded shadow mapping ****

csm_ground_height = -2.0
csm_sphere_scale = 1.0
csm_box_scale = 0.8

cascaded_shadow_mapping_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(3e3),
        UnitVector3.from((0.0, -0.08, 1.0)),
        0.0,
    )

cascaded_shadow_mapping_soft_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(3e3),
        UnitVector3.from((0.0, -0.08, 1.0)),
        1.5,
    )

cascaded_shadow_mapping_ground =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(20.0)
    |> Comp.ReferenceFrame.add_unoriented((0, csm_ground_height, 10))
    |> add_diffuse

cascaded_shadow_mapping_sphere =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(sphere_rings)
    |> Comp.ModelTransform.add_with_scale(csm_sphere_scale)
    |> Comp.ReferenceFrame.add_unoriented(
        (0.8, csm_ground_height + csm_sphere_scale / 2, 4.0),
    )
    |> add_plastic

cascaded_shadow_mapping_cylinder =
    Entity.new_component_data
    |> Setup.CylinderMesh.add_new(1.5, 0.2, 15)
    |> Comp.ReferenceFrame.add_unoriented((-1.0, csm_ground_height, 2.0))
    |> add_plastic

cascaded_shadow_mapping_box =
    Entity.new_component_data
    |> Setup.BoxMesh.add_unit_cube
    |> Comp.ModelTransform.add_with_scale(csm_box_scale)
    |> Comp.ReferenceFrame.add_unoriented(
        (0.0, csm_ground_height + csm_box_scale / 2, 10.0),
    )
    |> add_plastic
