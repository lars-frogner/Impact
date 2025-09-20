module [
    entity_ids,
    setup!,
    create_entities!,
]

import core.Point3
import pf.Entity
import pf.Setup.BoxMesh
import pf.Comp.ReferenceFrame
import pf.Setup.UniformColor
import pf.Setup.DynamicRigidBodySubstance
import pf.Setup.UniformSpecularReflectance
import pf.Comp.Motion
import pf.Physics.AngularVelocity as AngularVelocity
import Scenes.Blank

entity_ids = {
    major_axis_body: Entity.id("major_axis_body"),
    intermediate_axis_body: Entity.id("intermediate_axis_body"),
    minor_axis_body: Entity.id("minor_axis_body"),
}

setup! = |_|
    Scenes.Blank.setup!({})?
    create_entities!((0, 3, 8), 5.0, 1e-3)

create_entities! = |position, angular_speed, angular_velocity_perturbation_fraction|
    major_axis_body_position = Point3.displace(position, (5.0, 0.0, 0.0))
    intermediate_axis_body_position = position
    minor_axis_body_position = Point3.displace(position, (-5.0, 0.0, 0.0))

    angular_velocity_perturbation = angular_speed * angular_velocity_perturbation_fraction

    body_base =
        Entity.new_component_data
        |> Setup.BoxMesh.add_new(3, 2, 1, Outside)
        |> Setup.DynamicRigidBodySubstance.add({ mass_density: 1 / 6 })
        |> Setup.UniformColor.add((0.1, 0.1, 0.7))
        |> Setup.UniformSpecularReflectance.add_in_range_of(
            Setup.UniformSpecularReflectance.plastic,
            80.0,
        )

    major_axis_body =
        body_base
        |> Comp.ReferenceFrame.add_unoriented(major_axis_body_position)
        |> Comp.Motion.add_angular(
            AngularVelocity.from_vector(
                (
                    angular_velocity_perturbation,
                    angular_velocity_perturbation,
                    angular_speed,
                ),
            ),
        )

    intermediate_axis_body =
        body_base
        |> Comp.ReferenceFrame.add_unoriented(intermediate_axis_body_position)
        |> Comp.Motion.add_angular(
            AngularVelocity.from_vector(
                (
                    angular_velocity_perturbation,
                    angular_speed,
                    angular_velocity_perturbation,
                ),
            ),
        )

    minor_axis_body =
        body_base
        |> Comp.ReferenceFrame.add_unoriented(minor_axis_body_position)
        |> Comp.Motion.add_angular(
            AngularVelocity.from_vector(
                (
                    angular_speed,
                    angular_velocity_perturbation,
                    angular_velocity_perturbation,
                ),
            ),
        )

    Entity.create_with_id!(major_axis_body, entity_ids.major_axis_body)?
    Entity.create_with_id!(intermediate_axis_body, entity_ids.intermediate_axis_body)?
    Entity.create_with_id!(minor_axis_body, entity_ids.minor_axis_body)?

    Ok({})
