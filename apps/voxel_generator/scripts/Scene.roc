module [
    entity_ids,
    setup!,
]

import core.Radians
import core.UnitQuaternion
import core.UnitVector3
import core.Vector3
import pf.Entity
import pf.Comp.AmbientEmission
import pf.Comp.VelocityControl
import pf.Comp.AngularVelocityControl
import pf.Setup.PerspectiveCamera
import pf.Comp.ReferenceFrame
import pf.Comp.ShadowableOmnidirectionalEmission
import pf.Comp.ShadowableUnidirectionalEmission
import pf.Comp.Motion

entity_ids = {
    camera: Entity.id("camera"),
    object: Entity.id("object"),
}

setup! = |_|
    Entity.create_with_id!(camera, entity_ids.camera)?
    _ = Entity.create!(key_light)?
    _ = Entity.create!(fill_light)?
    _ = Entity.create!(rim_light)?
    _ = Entity.create!(ambient_light)?

    Ok({})

camera =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_new(
        (0.0, 15.0, 30.0),
        UnitQuaternion.from_axis_angle(UnitVector3.x_axis, -0.4),
    )
    |> Comp.Motion.add_stationary
    |> Comp.VelocityControl.add
    |> Comp.AngularVelocityControl.add_all_directions
    |> Setup.PerspectiveCamera.add_new(Radians.from_degrees(70), 0.01, 1000)

## Key light (directional) – strong, angled from above-right
key_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(50000), ## 50k lux – bright, like direct sun
        UnitVector3.from((0.6, -0.5, -1.0)), ## right, above, toward object
        2.0, ## 2° extent = soft sun disc
    )

## Fill light (point) – dimmer, opposite side
fill_light =
    Entity.new_component_data
    |> Comp.ReferenceFrame.add_unoriented((-10.0, 10.0, 15.0))
    |> Comp.ShadowableOmnidirectionalEmission.add_new(
        Vector3.same(5000), ## ~5000 cd (low intensity fill)
        0.5, ## source extent in m (soft falloff)
    )

## Rim light (directional) – from behind to create edge highlight
rim_light =
    Entity.new_component_data
    |> Comp.ShadowableUnidirectionalEmission.add_new(
        Vector3.same(20000), ## 20k lux
        UnitVector3.from((-0.8, 0.3, 1.0)), ## behind-left, slightly above
        3.0, ## slightly larger source
    )

## Optional ambient to lift shadows
ambient_light =
    Entity.new_component_data
    |> Comp.AmbientEmission.add_new(Vector3.same(10000)) ## 10000 lux, neutral base
