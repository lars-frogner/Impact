module [
    entity_ids,
    setup!,
    create_entities!,
]

import core.Point3
import core.UnitQuaternion
import core.UnitVector3
import pf.Entity
import pf.Setup.BoxMesh
import pf.Comp.HarmonicOscillatorTrajectory
import pf.Comp.ReferenceFrame
import pf.Setup.SphereMesh
import pf.Comp.Spring
import pf.Setup.UniformColor
import pf.Comp.UniformRigidBody
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Velocity
import pf.Physics.Spring
import Scenes.Blank

entity_ids = {
    attachment_point: Entity.id("attachment_point"),
    dynamic_body: Entity.id("dynamic_body"),
    spring: Entity.id("spring"),
    kinematic_body: Entity.id("kinematic_body"),
}

setup! = |_|
    Scenes.Blank.setup!({})?
    create_entities!((1.0, 7.0, 12.0), 1.0, 10.0, 3.0)

create_entities! = |position, mass, spring_constant, amplitude|
    angular_frequency = Num.sqrt(spring_constant / mass)
    period = Num.tau / angular_frequency

    attachment_position = position
    mass_position = Point3.displace(attachment_position, (0.0, -2.0 * amplitude - 0.5, 0.0))

    reference_position = Point3.displace(attachment_position, (-2.0, (-amplitude) - 0.5, 0.0))

    attachment_point =
        Entity.new
        |> Setup.SphereMesh.add_new(15)
        |> Comp.ReferenceFrame.add_unoriented_scaled(attachment_position, 0.2)
        |> Setup.UniformColor.add((0.8, 0.1, 0.1))

    dynamic_body =
        Entity.new
        |> Setup.BoxMesh.add_unit_cube
        |> Comp.UniformRigidBody.add({ mass_density: mass })
        |> Comp.ReferenceFrame.add_for_unoriented_rigid_body(mass_position)
        |> Comp.Velocity.add_stationary
        |> Setup.UniformColor.add((0.1, 0.1, 0.7))
        |> Setup.UniformSpecularReflectance.add_in_range_of(
            Setup.UniformSpecularReflectance.plastic,
            80.0,
        )

    spring =
        Entity.new
        |> Comp.ReferenceFrame.add_unoriented(Point3.origin)
        |> Comp.Spring.add_new(
            entity_ids.attachment_point,
            entity_ids.dynamic_body,
            Point3.origin,
            Point3.origin,
            Physics.Spring.standard(spring_constant, 0, amplitude + 0.5),
        )

    kinematic_body =
        Entity.new
        |> Setup.BoxMesh.add_unit_cube
        |> Comp.ReferenceFrame.add_for_driven_trajectory(UnitQuaternion.identity)
        |> Comp.Velocity.add_stationary
        |> Comp.HarmonicOscillatorTrajectory.add_new(
            0.25 * period,
            reference_position,
            UnitVector3.y_axis,
            amplitude,
            period,
        )
        |> Setup.UniformColor.add((0.1, 0.7, 0.1))
        |> Setup.UniformSpecularReflectance.add_in_range_of(
            Setup.UniformSpecularReflectance.plastic,
            80.0,
        )

    Entity.create_with_id!(entity_ids.attachment_point, attachment_point)?
    Entity.create_with_id!(entity_ids.dynamic_body, dynamic_body)?
    Entity.create_with_id!(entity_ids.spring, spring)?
    Entity.create_with_id!(entity_ids.kinematic_body, kinematic_body)?

    Ok({})
