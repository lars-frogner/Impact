module [
    entity_ids,
    setup!,
]

import core.Plane
import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Entity
import pf.Comp.AmbientEmission
import pf.Comp.ControlledVelocity
import pf.Comp.ControlledAngularVelocity
import pf.Setup.PerspectiveCamera
import pf.Setup.PlanarCollidable
import pf.Setup.RectangleMesh
import pf.Comp.ReferenceFrame
import pf.Comp.ModelTransform
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Setup.UniformColor
import pf.Setup.UniformRoughness
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Physics.ContactResponseParameters

entity_ids = {
    player: Entity.id("player"),
    ground: Entity.id("ground"),
    ambient_light: Entity.id("ambient_light"),
    unidirectional_light: Entity.id("unidirectional_light"),
}

setup! : {} => Result {} Str
setup! = |_|
    Entity.create_with_id!(player, entity_ids.player)?
    Entity.create_with_id!(ground, entity_ids.ground)?
    Entity.create_with_id!(ambient_light, entity_ids.ambient_light)?
    Entity.create_with_id!(unidirectional_light, entity_ids.unidirectional_light)?

    Ok({})

player =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, 2.0, 0.0),
        UnitQuaternion.from_axis_angle(UnitVector3.y_axis, Num.pi),
    )
    |> Comp.Motion.add_stationary
    |> Comp.ControlledVelocity.add_new
    |> Comp.ControlledAngularVelocity.add_new
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

ground =
    Entity.new_component_data
    |> Setup.RectangleMesh.add_unit_square
    |> Comp.ModelTransform.add_with_scale(1000)
    |> Setup.PlanarCollidable.add_new(
        Static,
        Plane.new(UnitVector3.y_axis, 0.0),
        Physics.ContactResponseParameters.new(0.0, 0.0, 0.0),
    )
    |> Setup.UniformColor.add((0.9, 0.9, 0.9))
    |> Setup.UniformSpecularReflectance.add(0.01)
    |> Setup.UniformRoughness.add(0.5)

ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(2000000))

unidirectional_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(200000),
        UnitVector3.from((0.0, -1.0, 0.0)),
        2.0,
    )
