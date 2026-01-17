module [
    entity_ids,
    camera,
    spawn!,
]

import core.Radians
import core.UnitQuaternion exposing [UnitQuaternion]
import core.UnitVector3
import core.Vector3
import core.Matrix3
import core.Point3 exposing [Point3]
import core.Sphere

import pf.Entity

import pf.Setup.LocalForce
import pf.Setup.CylinderMesh
import pf.Setup.SphereMesh
import pf.Setup.CapsuleMesh
import pf.Setup.DynamicRigidBodyInertialProperties
import pf.Comp.AngularVelocityControl
import pf.Comp.AngularVelocityControlParent
import pf.Control.AngularVelocityControlDirections
import pf.Control.AngularVelocityControlFlags
import pf.Setup.SceneParent
import pf.Setup.PerspectiveCamera
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Setup.SceneGraphGroup
import pf.Setup.SphereMesh
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformEmissiveLuminance
import pf.Comp.Motion
import pf.Comp.VoxelAbsorbingCapsule
import pf.Comp.VoxelAbsorbingSphere
import pf.Comp.SceneEntityFlags
import pf.Physics.ContactResponseParameters
import pf.Setup.LocalForce
import pf.Setup.SphericalCollidable
import pf.Comp.DynamicGravity
import pf.Setup.GravityAlignmentTorque

entity_ids = {
    camera: Entity.id("overview_camera"),
}

camera = {
    field_of_view: 70,
    near_distance: 0.01,
    view_distance: 2e5,
}

spawn! : F32 => Result {} Str
spawn! = |radius_to_cover|
    camera_ent = construct_entity(radius_to_cover)
    Entity.create_with_id!(camera_ent, entity_ids.camera)?
    Ok({})

construct_entity : F32 -> Entity.ComponentData
construct_entity = |radius_to_cover|
    height = height_to_cover_radius(radius_to_cover)

    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, height, 0.0),
        UnitQuaternion.mul(
            UnitQuaternion.from_axis_angle(UnitVector3.unit_y, Num.pi),
            UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2),
        ),
    )
    |> Setup.PerspectiveCamera.add_new(
        Radians.from_degrees(camera.field_of_view),
        camera.near_distance,
        camera.view_distance,
    )

height_to_cover_radius = |radius_to_cover|
    radius_to_cover / Num.tan(0.5 * Radians.from_degrees(camera.field_of_view))
