module [
    entity_ids,
    setup!,
    set_medium!,
    create_entities!,
]

import core.Point3
import core.UnitQuaternion
import core.UnitVector3
import pf.Command
import pf.Entity
import pf.Comp.ConeMesh
import pf.Comp.DetailedDrag
import pf.Comp.LogsKineticEnergy
import pf.Comp.LogsMomentum
import pf.Comp.ReferenceFrame
import pf.Comp.UniformColor
import pf.Comp.UniformGravity
import pf.Comp.UniformRigidBody
import pf.Comp.UniformSpecularReflectance
import pf.Comp.Velocity
import pf.Physics.UniformMedium
import Scenes.Blank

entity_ids = {
    cone_with_drag: Entity.id("cone_with_drag"),
    cone_without_drag: Entity.id("cone_without_drag"),
}

setup! = |_|
    Scenes.Blank.setup!({})?
    set_medium!({})?
    create_entities!((0, 25, 30))

set_medium! = |_|
    Command.execute!(Physics(SetMedium(Physics.UniformMedium.moving_air((0, 3, 0)))))

create_entities! = |position|
    cone_base =
        Entity.new
        |> Comp.ConeMesh.add_new(2, 1, 100)
        |> Comp.UniformRigidBody.add({ mass_density: 10 })
        |> Comp.Velocity.add_stationary
        |> Comp.UniformSpecularReflectance.add_in_range_of(
            Comp.UniformSpecularReflectance.plastic,
            80.0,
        )
        |> Comp.UniformGravity.add_earth
        |> Comp.LogsKineticEnergy.add
        |> Comp.LogsMomentum.add

    cone_with_drag =
        cone_base
        |> Comp.ReferenceFrame.add_for_rigid_body(
            position,
            UnitQuaternion.from_axis_angle(UnitVector3.z_axis, 3.0),
        )
        |> Comp.UniformColor.add((0.1, 0.1, 0.7))
        |> Comp.DetailedDrag.add_new(1.0)

    cone_without_drag =
        cone_base
        |> Comp.ReferenceFrame.add_for_rigid_body(
            Point3.displace(position, (-5, 0, 0)),
            UnitQuaternion.from_axis_angle(UnitVector3.z_axis, 3.0),
        )
        |> Comp.UniformColor.add((0.7, 0.1, 0.1))

    Entity.create_with_id!(entity_ids.cone_with_drag, cone_with_drag)?
    Entity.create_with_id!(entity_ids.cone_without_drag, cone_without_drag)?

    Ok({})
