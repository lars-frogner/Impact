module [
    entity_ids,
    setup!,
    handle_keyboard_event!,
    handle_mouse_button_event!,
]

import core.Radians
import core.Plane
import core.UnitQuaternion
import core.UnitVector3 exposing [unit_y]
import core.Vector3
import core.Point3
import core.Sphere
import pf.Command
import pf.Entity
import pf.Comp.AmbientEmission
import pf.Setup.CylinderMesh
import pf.Setup.DynamicRigidBodySubstance
import pf.Setup.SameVoxelType
import pf.Comp.VelocityControl
import pf.Comp.AngularVelocityControl
import pf.Comp.ParentEntity
import pf.Setup.PerspectiveCamera
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Comp.CanBeParent
import pf.Setup.SphereMesh
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Setup.VoxelAbsorbingCapsule
import pf.Setup.VoxelAbsorbingSphere
import pf.Setup.VoxelSphere
import pf.Setup.VoxelBox
import pf.Setup.VoxelCapsule
import pf.Setup.DynamicVoxels
import pf.Setup.ConstantAcceleration
import pf.Input.KeyboardEvent exposing [KeyboardEvent]
import pf.Input.MouseButtonEvent exposing [MouseButtonEvent]
import InputHandling.Keyboard as KeyboardInput
import InputHandling.MouseButton as MouseButtonInput
import pf.Physics.AngularVelocity as AngularVelocity
import pf.Comp.SceneEntityFlags
import pf.Setup.PlanarCollidable
import pf.Setup.SphericalCollidable
import pf.Setup.VoxelCollidable
import pf.Physics.ContactResponseParameters
import pf.Comp.FracturingProperties

entity_ids = {
    player: Entity.id("player"),
    camera: Entity.id("camera"),
    laser: Entity.id("laser"),
    absorbing_sphere: Entity.id("absorbing_sphere"),
    ground: Entity.id("ground"),
    fracture_object: Entity.id("fracture_object"),
    ambient_light: Entity.id("ambient_light"),
    omnidirectional_light: Entity.id("omnidirectional_light"),
    unidirectional_light: Entity.id("unidirectional_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Entity.create_with_id!(player, entity_ids.player)?
    Entity.create_with_id!(camera, entity_ids.camera)?
    Entity.create_with_id!(laser, entity_ids.laser)?
    Entity.create_with_id!(absorbing_sphere, entity_ids.absorbing_sphere)?
    Entity.create_with_id!(ground, entity_ids.ground)?
    Entity.create_with_id!(ambient_light, entity_ids.ambient_light)?
    Entity.create_with_id!(omnidirectional_light, entity_ids.omnidirectional_light)?
    Entity.create_with_id!(unidirectional_light, entity_ids.unidirectional_light)?

    setup_experiment!(DroppedBox)

setup_experiment! = |experiment|
    when experiment is
        DroppedBox ->
            setup_dropped_box!({})
        DroppedSphere ->
            setup_dropped_sphere!({})
        HeadOnSpheres ->
            setup_colliding_spheres!(0)
        OffsetSpheres ->
            setup_colliding_spheres!(0.5)
        BoxImpactingSphere ->
            setup_box_impacting_sphere!({})
        SimultaneousSideImpacts ->
            setup_simultaneous_side_impacts!({})
        BulletImpactingTargets ->
            setup_bullet_impacting_targets!({})
        RotationalImpact ->
            setup_rotational_impact!({})

setup_dropped_box! = |{}|
    response_params = {
        restitution_coef: 0.4,
        static_friction_coef: 0.7,
        dynamic_friction_coef: 0.5,
    }
    fracturing_props = {
        force_threshold : 5e4,
        fragment_scale : 0.3,
        min_fragment_extent : 0.15,
        max_fragment_extent : 0.3,
    }
    base_object(response_params, fracturing_props)
    |> Setup.VoxelBox.add_new(0.04, 50, 50, 50)
    |> Comp.ReferenceFrame.add_new(
        (0, 5, 5),
        UnitQuaternion.mul(
            UnitQuaternion.from_axis_angle(UnitVector3.unit_z, 1.0),
            UnitQuaternion.from_axis_angle(UnitVector3.unit_x, 1.0),
        ),
    )
    |> Setup.ConstantAcceleration.add_earth
    |> Entity.create!
    |> Result.map_ok(|_| {})

setup_dropped_sphere! = |{}|
    response_params = {
        restitution_coef: 0.4,
        static_friction_coef: 0.7,
        dynamic_friction_coef: 0.5,
    }
    fracturing_props = {
        force_threshold : 5e4,
        fragment_scale : 0.3,
        min_fragment_extent : 0.15,
        max_fragment_extent : 0.3,
    }
    base_object(response_params, fracturing_props)
    |> Setup.VoxelSphere.add_new(0.04, 30)
    |> Comp.ReferenceFrame.add_unoriented((0, 5, 5))
    |> Setup.ConstantAcceleration.add_earth
    |> Entity.create!
    |> Result.map_ok(|_| {})

setup_colliding_spheres! = |y_offset|
    response_params = {
        restitution_coef: 0.4,
        static_friction_coef: 0.7,
        dynamic_friction_coef: 0.5,
    }
    fracturing_props = {
        force_threshold : 1e5,
        fragment_scale : 0.3,
        min_fragment_extent : 0.15,
        max_fragment_extent : 0.3,
    }
    sphere =  base_object(response_params, fracturing_props)
        |> Setup.VoxelSphere.add_new(0.04, 20)

    rel_speed = 10

    sphere
    |> Comp.ReferenceFrame.add_unoriented((-3, 1 - y_offset / 2, 4))
    |> Comp.Motion.add_linear((rel_speed / 2, 0, 0))
    |> Entity.create!
    |> Result.map_ok(|_| {})?

    sphere
    |> Comp.ReferenceFrame.add_unoriented((3, 1 + y_offset / 2, 4))
    |> Comp.Motion.add_linear((-rel_speed / 2, 0, 0))
    |> Entity.create!
    |> Result.map_ok(|_| {})

setup_box_impacting_sphere! = |{}|
    response_params = {
        restitution_coef: 0.4,
        static_friction_coef: 0.7,
        dynamic_friction_coef: 0.5,
    }
    fracturing_props = {
        force_threshold : 5e4,
        fragment_scale : 0.3,
        min_fragment_extent : 0.15,
        max_fragment_extent : 0.3,
    }
    base =  base_object(response_params, fracturing_props)

    y_offset = 0.8
    speed = 8

    base
    |> Setup.VoxelSphere.add_new(0.04, 30)
    |> Comp.ReferenceFrame.add_unoriented((0, 1 - y_offset / 2, 5))
    |> Comp.Motion.add_stationary
    |> Entity.create!
    |> Result.map_ok(|_| {})?

    base
    |> Setup.VoxelBox.add_new(0.04, 30, 30, 30)
    |> Comp.ReferenceFrame.add_unoriented((-5, 1 + y_offset / 2, 5))
    |> Comp.Motion.add_linear((speed, 0, 0))
    |> Entity.create!
    |> Result.map_ok(|_| {})

setup_simultaneous_side_impacts! = |{}|
    response_params = {
        restitution_coef: 0.4,
        static_friction_coef: 0.7,
        dynamic_friction_coef: 0.5,
    }
    fracturing_props = {
        force_threshold : 1e5,
        fragment_scale : 0.3,
        min_fragment_extent : 0.15,
        max_fragment_extent : 0.3,
    }

    sphere =  base_object(response_params, fracturing_props)

    rel_speed = 12

    sphere
    |> Setup.VoxelSphere.add_new(0.04, 30)
    |> Comp.ReferenceFrame.add_unoriented((0, 1, 4))
    |> Comp.Motion.add_linear((0, 0, 0))
    |> Entity.create!
    |> Result.map_ok(|_| {})?

    sphere
    |> Setup.VoxelSphere.add_new(0.04, 15)
    |> Comp.ReferenceFrame.add_unoriented((-3, 1, 4))
    |> Comp.Motion.add_linear((rel_speed / 2, 0, 0))
    |> Entity.create!
    |> Result.map_ok(|_| {})?

    sphere
    |> Setup.VoxelSphere.add_new(0.04, 15)
    |> Comp.ReferenceFrame.add_unoriented((3, 1, 4))
    |> Comp.Motion.add_linear((-rel_speed / 2, 0, 0))
    |> Entity.create!
    |> Result.map_ok(|_| {})

setup_bullet_impacting_targets! = |{}|
    response_params = {
        restitution_coef: 0.4,
        static_friction_coef: 0.7,
        dynamic_friction_coef: 0.5,
    }
    fracturing_props = {
        force_threshold : 1e5,
        fragment_scale : 0.3,
        min_fragment_extent : 0.15,
        max_fragment_extent : 0.3,
    }
    base =  base_object(response_params, fracturing_props)

    speed = 10

    base
    |> Setup.VoxelBox.add_new(0.04, 50, 50, 50)
    |> Comp.ReferenceFrame.add_unoriented((1.5, 1, 5))
    |> Comp.Motion.add_stationary
    |> Entity.create!
    |> Result.map_ok(|_| {})?

    base
    |> Setup.VoxelSphere.add_new(0.04, 30)
    |> Comp.ReferenceFrame.add_unoriented((-1.5, 1, 5))
    |> Comp.Motion.add_stationary
    |> Entity.create!
    |> Result.map_ok(|_| {})?

    _bullet =
        Entity.new_component_data
        |> Comp.ReferenceFrame.add_unoriented((-5, 1, 5))
        |> Comp.Motion.add_linear((speed, 0, 0))
        |> Setup.SphereMesh.add_new(32)
        |> Comp.ModelTransform.add_with_scale(0.2)
        |> Setup.UniformColor.add((0.8, 0.2, 0.1))
        |> Setup.UniformSpecularReflectance.add_in_range_of(
            Setup.UniformSpecularReflectance.plastic,
            0.0,
        )
        |> Setup.UniformRoughness.add(0.55)
        |> Setup.DynamicRigidBodySubstance.add({ mass_density: 1e6 })
        |> Setup.SphericalCollidable.add_new(
            Dynamic,
            Sphere.new(Point3.origin, 1.0),
            response_params,
        )
        |> Entity.create!
        |> Result.map_ok(|_| {})?

    Ok({})

setup_rotational_impact! = |{}|
    response_params = {
        restitution_coef: 0.4,
        static_friction_coef: 0.7,
        dynamic_friction_coef: 0.5,
    }
    fracturing_props = {
        force_threshold : 5e4,
        fragment_scale : 0.3,
        min_fragment_extent : 0.15,
        max_fragment_extent : 0.3,
    }
    base =  base_object(response_params, fracturing_props)

    base
    |> Setup.VoxelSphere.add_new(0.04, 20)
    |> Comp.ReferenceFrame.add_unoriented((0.5, 1, 5))
    |> Comp.Motion.add_stationary
    |> Entity.create!
    |> Result.map_ok(|_| {})?

    base
    |> Setup.VoxelCapsule.add_new(0.04, 60, 10)
    |> Comp.ReferenceFrame.add_unoriented((-0.8, 1.5, 5))
    |> Comp.Motion.add_new(
        (0, 0, 0),
        AngularVelocity.new(UnitVector3.unit_z, Radians.from_degrees(-140.0)),
    )
    |> Entity.create!
    |> Result.map_ok(|_| {})

base_object = |response_params, fracturing_props|
    Entity.new_component_data
    |> Setup.SameVoxelType.add_new("Default")
    |> Setup.DynamicVoxels.add
    |> Setup.VoxelCollidable.add_new(
        Dynamic,
        response_params,
    )
    |> Comp.FracturingProperties.add(fracturing_props)

handle_keyboard_event! : KeyboardEvent => Result {} Str
handle_keyboard_event! = |{ key, state }|
    command =
        when key is
            Letter(letter_key) ->
                when letter_key is
                    KeyF ->
                        KeyboardInput.on_released(
                            state,
                            App(
                                FractureVoxelObject {
                                    entity_id: entity_ids.fracture_object,
                                    points_per_dim: 5,
                                },
                            ),
                        )

                    _ -> None

            _ -> None

    when command is
        Some(comm) -> Command.execute!(comm)
        None -> KeyboardInput.handle_event!({ key, state })

handle_mouse_button_event! : MouseButtonEvent => Result {} Str
handle_mouse_button_event! = |{ button, state }|
    when button is
        Left ->
            MouseButtonInput.toggle_scene_entity_active_state!(
                entity_ids.laser,
                state,
            )

        Right ->
            MouseButtonInput.toggle_scene_entity_active_state!(
                entity_ids.absorbing_sphere,
                state,
            )

        _ -> Ok({})

player =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, 1.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.unit_y, Num.pi),
    )
    |> Comp.Motion.add_stationary
    |> Comp.VelocityControl.add
    |> Comp.AngularVelocityControl.add_all_directions
    |> Comp.CanBeParent.add

camera =
    Entity.new_component_data
    |> Comp.ParentEntity.add(entity_ids.player)
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

laser =
    Entity.new_component_data
    |> Comp.ParentEntity.add(entity_ids.player)
    |> Comp.ReferenceFrame.add_new(
        (0.15, -0.3, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2),
    )
    |> Setup.CylinderMesh.add_new(100, 0.02, 16)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Setup.VoxelAbsorbingCapsule.add_new(Vector3.same(0), (0, 100, 0), 0.3)
    |> Comp.SceneEntityFlags.add(
        Comp.SceneEntityFlags.union(
            Comp.SceneEntityFlags.is_disabled,
            Comp.SceneEntityFlags.casts_no_shadows,
        ),
    )

absorbing_sphere =
    Entity.new_component_data
    |> Comp.ParentEntity.add(entity_ids.player)
    |> Comp.ModelTransform.add_with_scale(0.05)
    |> Comp.ReferenceFrame.add_unoriented((0, 0, -3))
    |> Setup.SphereMesh.add_new(64)
    |> Setup.UniformColor.add((0.9, 0.05, 0.05))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(Vector3.scale((1.0, 0.2, 0.2), 1e5), 0.2)
    |> Setup.VoxelAbsorbingSphere.add_new(Vector3.same(0), 1)
    |> Comp.SceneEntityFlags.add(Comp.SceneEntityFlags.is_disabled)

ground =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(20)
    |> Comp.ReferenceFrame.add_unoriented((0, -1, 0))
    |> Comp.Motion.add_stationary
    |> Setup.UniformColor.add((1, 1, 1))
    |> Setup.UniformSpecularReflectance.add(0.01)
    |> Setup.UniformRoughness.add(0.5)
    |> Setup.PlanarCollidable.add_new(
        Static,
        Plane.new(unit_y, 0),
        Physics.ContactResponseParameters.new(0.0, 0.7, 0.5),
    )

ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(1e5))

omnidirectional_light =
    Entity.new_component_data
    |> Setup.SphereMesh.add_new(25)
    |> Comp.ModelTransform.add_with_scale(0.35)
    |> Comp.ReferenceFrame.add_unoriented((5, 5, 0))
    |> Setup.UniformColor.add((1, 1, 1))
    |> Setup.UniformEmissiveLuminance.add(1e6)
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(2e7),
        0.7,
    )

unidirectional_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(2e6),
        UnitVector3.from((0.0, -1.0, 0.0)),
        2.0,
    )
