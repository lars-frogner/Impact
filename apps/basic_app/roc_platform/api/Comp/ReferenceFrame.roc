# Hash: 184dd8519c579332dd2a5f08fface71b19009187f709758535bfe0b4abb78060
# Generated: 2025-05-24T10:33:40+00:00
# Rust type: impact::physics::motion::components::ReferenceFrameComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    ReferenceFrame,
    new,
    unscaled,
    unoriented,
    unoriented_scaled,
    unlocated,
    unlocated_scaled,
    scaled,
    unoriented_with_offset_origin,
    unoriented_scaled_with_offset_origin,
    scaled_with_offset_origin,
    with_offset_origin,
    for_rigid_body,
    for_scaled_rigid_body,
    for_unoriented_rigid_body,
    for_scaled_unoriented_rigid_body,
    for_driven_rotation,
    for_scaled_driven_rotation,
    for_driven_rotation_around_offset_origin,
    for_scaled_driven_rotation_around_offset_origin,
    for_driven_trajectory,
    for_scaled_driven_trajectory,
    for_driven_trajectory_with_offset_origin,
    for_scaled_driven_trajectory_with_offset_origin,
    for_driven_trajectory_and_rotation,
    for_scaled_driven_trajectory_and_rotation,
    for_driven_trajectory_and_rotation_with_offset_origin,
    for_scaled_driven_trajectory_and_rotation_with_offset_origin,
    add_new,
    add_multiple_new,
    add_unscaled,
    add_multiple_unscaled,
    add_unoriented,
    add_multiple_unoriented,
    add_unoriented_scaled,
    add_multiple_unoriented_scaled,
    add_unlocated,
    add_multiple_unlocated,
    add_unlocated_scaled,
    add_multiple_unlocated_scaled,
    add_scaled,
    add_multiple_scaled,
    add_unoriented_with_offset_origin,
    add_multiple_unoriented_with_offset_origin,
    add_unoriented_scaled_with_offset_origin,
    add_multiple_unoriented_scaled_with_offset_origin,
    add_scaled_with_offset_origin,
    add_multiple_scaled_with_offset_origin,
    add_with_offset_origin,
    add_multiple_with_offset_origin,
    add_for_rigid_body,
    add_multiple_for_rigid_body,
    add_for_scaled_rigid_body,
    add_multiple_for_scaled_rigid_body,
    add_for_unoriented_rigid_body,
    add_multiple_for_unoriented_rigid_body,
    add_for_scaled_unoriented_rigid_body,
    add_multiple_for_scaled_unoriented_rigid_body,
    add_for_driven_rotation,
    add_multiple_for_driven_rotation,
    add_for_scaled_driven_rotation,
    add_multiple_for_scaled_driven_rotation,
    add_for_driven_rotation_around_offset_origin,
    add_multiple_for_driven_rotation_around_offset_origin,
    add_for_scaled_driven_rotation_around_offset_origin,
    add_multiple_for_scaled_driven_rotation_around_offset_origin,
    add_for_driven_trajectory,
    add_multiple_for_driven_trajectory,
    add_for_scaled_driven_trajectory,
    add_multiple_for_scaled_driven_trajectory,
    add_for_driven_trajectory_with_offset_origin,
    add_multiple_for_driven_trajectory_with_offset_origin,
    add_for_scaled_driven_trajectory_with_offset_origin,
    add_multiple_for_scaled_driven_trajectory_with_offset_origin,
    add_for_driven_trajectory_and_rotation,
    add_multiple_for_driven_trajectory_and_rotation,
    add_for_scaled_driven_trajectory_and_rotation,
    add_multiple_for_scaled_driven_trajectory_and_rotation,
    add_for_driven_trajectory_and_rotation_with_offset_origin,
    add_multiple_for_driven_trajectory_and_rotation_with_offset_origin,
    add_for_scaled_driven_trajectory_and_rotation_with_offset_origin,
    add_multiple_for_scaled_driven_trajectory_and_rotation_with_offset_origin,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Point3
import core.UnitQuaternion
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that have a
## reference frame defined by position, orientation and scaling.
ReferenceFrame : {
    ## The offset, expressed in the entity's reference frame (before scaling),
    ## from the original origin of the entity's reference frame to the point
    ## that should be used as the actual origin.
    origin_offset : Vector3.Vector3 Binary64,
    ## The coordinates of the origin of the entity's reference frame measured
    ## in the parent space.
    position : Point3.Point3 Binary64,
    ## The 3D orientation of the entity's reference frame in the parent space.
    orientation : UnitQuaternion.UnitQuaternion Binary64,
    ## The uniform scale factor of the entity's reference frame (distance in
    ## world space per distance in the reference frame).
    scaling : F64,
}

## Creates a new reference frame component with the given position,
## orientation and scaling, retaining the original origin of the entity's
## reference frame.
new : Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
new = |position, orientation, scaling|
    { origin_offset: Vector3.zero, position, orientation, scaling }

## Creates a new reference frame component with the given position,
## orientation and scaling, retaining the original origin of the entity's
## reference frame.
## Adds the component to the given entity's data.
add_new : Entity.Data, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_new = |entity_data, position, orientation, scaling|
    add(entity_data, new(position, orientation, scaling))

## Creates a new reference frame component with the given position,
## orientation and scaling, retaining the original origin of the entity's
## reference frame.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, position, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            position, orientation, scaling,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Creates a new reference frame component with the given position and
## orientation, retaining the original origin of the entity's reference
## frame and no scaling.
unscaled : Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
unscaled = |position, orientation|
    new(position, orientation, 1.0)

## Creates a new reference frame component with the given position and
## orientation, retaining the original origin of the entity's reference
## frame and no scaling.
## Adds the component to the given entity's data.
add_unscaled : Entity.Data, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_unscaled = |entity_data, position, orientation|
    add(entity_data, unscaled(position, orientation))

## Creates a new reference frame component with the given position and
## orientation, retaining the original origin of the entity's reference
## frame and no scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unscaled : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_unscaled = |entity_data, position, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, orientation,
            Entity.multi_count(entity_data),
            unscaled
        ))
    )

## Creates a new reference frame component with the given position,
## retaining the original origin of the entity's reference frame and the
## identity orientation and scaling.
unoriented : Point3.Point3 Binary64 -> ReferenceFrame
unoriented = |position|
    unoriented_scaled(position, 1.0)

## Creates a new reference frame component with the given position,
## retaining the original origin of the entity's reference frame and the
## identity orientation and scaling.
## Adds the component to the given entity's data.
add_unoriented : Entity.Data, Point3.Point3 Binary64 -> Entity.Data
add_unoriented = |entity_data, position|
    add(entity_data, unoriented(position))

## Creates a new reference frame component with the given position,
## retaining the original origin of the entity's reference frame and the
## identity orientation and scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64) -> Result Entity.MultiData Str
add_multiple_unoriented = |entity_data, position|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            position,
            Entity.multi_count(entity_data),
            unoriented
        ))
    )

## Creates a new reference frame component with the given position and
## scaling, retaining the original origin of the entity's reference frame
## and the identity orientation.
unoriented_scaled : Point3.Point3 Binary64, F64 -> ReferenceFrame
unoriented_scaled = |position, scaling|
    new(position, UnitQuaternion.identity, scaling)

## Creates a new reference frame component with the given position and
## scaling, retaining the original origin of the entity's reference frame
## and the identity orientation.
## Adds the component to the given entity's data.
add_unoriented_scaled : Entity.Data, Point3.Point3 Binary64, F64 -> Entity.Data
add_unoriented_scaled = |entity_data, position, scaling|
    add(entity_data, unoriented_scaled(position, scaling))

## Creates a new reference frame component with the given position and
## scaling, retaining the original origin of the entity's reference frame
## and the identity orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented_scaled : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_unoriented_scaled = |entity_data, position, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, scaling,
            Entity.multi_count(entity_data),
            unoriented_scaled
        ))
    )

## Creates a new reference frame component with the given orientation,
## retaining the original origin of the entity's reference frame and
## located at the origin with no scaling.
unlocated : UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
unlocated = |orientation|
    unlocated_scaled(orientation, 1.0)

## Creates a new reference frame component with the given orientation,
## retaining the original origin of the entity's reference frame and
## located at the origin with no scaling.
## Adds the component to the given entity's data.
add_unlocated : Entity.Data, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_unlocated = |entity_data, orientation|
    add(entity_data, unlocated(orientation))

## Creates a new reference frame component with the given orientation,
## retaining the original origin of the entity's reference frame and
## located at the origin with no scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unlocated : Entity.MultiData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_unlocated = |entity_data, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            orientation,
            Entity.multi_count(entity_data),
            unlocated
        ))
    )

## Creates a new reference frame component with the given orientation and
## scaling, retaining the original origin of the entity's reference frame
## and located at the origin.
unlocated_scaled : UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
unlocated_scaled = |orientation, scaling|
    new(Point3.origin, orientation, scaling)

## Creates a new reference frame component with the given orientation and
## scaling, retaining the original origin of the entity's reference frame
## and located at the origin.
## Adds the component to the given entity's data.
add_unlocated_scaled : Entity.Data, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_unlocated_scaled = |entity_data, orientation, scaling|
    add(entity_data, unlocated_scaled(orientation, scaling))

## Creates a new reference frame component with the given orientation and
## scaling, retaining the original origin of the entity's reference frame
## and located at the origin.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unlocated_scaled : Entity.MultiData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_unlocated_scaled = |entity_data, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            orientation, scaling,
            Entity.multi_count(entity_data),
            unlocated_scaled
        ))
    )

## Creates a new reference frame component with the given scaling,
## retaining the original origin of the entity's reference frame and
## located at the origin with the identity orientation.
scaled : F64 -> ReferenceFrame
scaled = |scaling|
    unoriented_scaled(Point3.origin, scaling)

## Creates a new reference frame component with the given scaling,
## retaining the original origin of the entity's reference frame and
## located at the origin with the identity orientation.
## Adds the component to the given entity's data.
add_scaled : Entity.Data, F64 -> Entity.Data
add_scaled = |entity_data, scaling|
    add(entity_data, scaled(scaling))

## Creates a new reference frame component with the given scaling,
## retaining the original origin of the entity's reference frame and
## located at the origin with the identity orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_scaled : Entity.MultiData, Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_scaled = |entity_data, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            scaling,
            Entity.multi_count(entity_data),
            scaled
        ))
    )

## Creates a new reference frame component with the given origin offset and
## position, and with the identity orientation and scaling.
unoriented_with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64 -> ReferenceFrame
unoriented_with_offset_origin = |origin_offset, position|
    unoriented_scaled_with_offset_origin(origin_offset, position, 1.0)

## Creates a new reference frame component with the given origin offset and
## position, and with the identity orientation and scaling.
## Adds the component to the given entity's data.
add_unoriented_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64 -> Entity.Data
add_unoriented_with_offset_origin = |entity_data, origin_offset, position|
    add(entity_data, unoriented_with_offset_origin(origin_offset, position))

## Creates a new reference frame component with the given origin offset and
## position, and with the identity orientation and scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64) -> Result Entity.MultiData Str
add_multiple_unoriented_with_offset_origin = |entity_data, origin_offset, position|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            origin_offset, position,
            Entity.multi_count(entity_data),
            unoriented_with_offset_origin
        ))
    )

## Creates a new reference frame component with the given origin offset,
## position and scaling, and with the identity orientation.
unoriented_scaled_with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64, F64 -> ReferenceFrame
unoriented_scaled_with_offset_origin = |origin_offset, position, scaling|
    scaled_with_offset_origin(origin_offset, position, UnitQuaternion.identity, scaling)

## Creates a new reference frame component with the given origin offset,
## position and scaling, and with the identity orientation.
## Adds the component to the given entity's data.
add_unoriented_scaled_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64, F64 -> Entity.Data
add_unoriented_scaled_with_offset_origin = |entity_data, origin_offset, position, scaling|
    add(entity_data, unoriented_scaled_with_offset_origin(origin_offset, position, scaling))

## Creates a new reference frame component with the given origin offset,
## position and scaling, and with the identity orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented_scaled_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_unoriented_scaled_with_offset_origin = |entity_data, origin_offset, position, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin_offset, position, scaling,
            Entity.multi_count(entity_data),
            unoriented_scaled_with_offset_origin
        ))
    )

## Creates a new reference frame component with the given origin offset,
## position orientation, and scaling.
scaled_with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
scaled_with_offset_origin = |origin_offset, position, orientation, scaling|
    { origin_offset, position, orientation, scaling }

## Creates a new reference frame component with the given origin offset,
## position orientation, and scaling.
## Adds the component to the given entity's data.
add_scaled_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_scaled_with_offset_origin = |entity_data, origin_offset, position, orientation, scaling|
    add(entity_data, scaled_with_offset_origin(origin_offset, position, orientation, scaling))

## Creates a new reference frame component with the given origin offset,
## position orientation, and scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_scaled_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_scaled_with_offset_origin = |entity_data, origin_offset, position, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            origin_offset, position, orientation, scaling,
            Entity.multi_count(entity_data),
            scaled_with_offset_origin
        ))
    )

## Creates a new reference frame component with the given origin offset,
## position and orientation and no scaling.
with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
with_offset_origin = |origin_offset, position, orientation|
    scaled_with_offset_origin(origin_offset, position, orientation, 1.0)

## Creates a new reference frame component with the given origin offset,
## position and orientation and no scaling.
## Adds the component to the given entity's data.
add_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_with_offset_origin = |entity_data, origin_offset, position, orientation|
    add(entity_data, with_offset_origin(origin_offset, position, orientation))

## Creates a new reference frame component with the given origin offset,
## position and orientation and no scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_with_offset_origin = |entity_data, origin_offset, position, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin_offset, position, orientation,
            Entity.multi_count(entity_data),
            with_offset_origin
        ))
    )

## Creates a new reference frame component with the given position and
## orientation for a rigid body and no scaling. The origin offset will be
## set to the center of mass.
for_rigid_body : Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
for_rigid_body = |position, orientation|
    for_scaled_rigid_body(position, orientation, 1.0)

## Creates a new reference frame component with the given position and
## orientation for a rigid body and no scaling. The origin offset will be
## set to the center of mass.
## Adds the component to the given entity's data.
add_for_rigid_body : Entity.Data, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_for_rigid_body = |entity_data, position, orientation|
    add(entity_data, for_rigid_body(position, orientation))

## Creates a new reference frame component with the given position and
## orientation for a rigid body and no scaling. The origin offset will be
## set to the center of mass.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_rigid_body : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_for_rigid_body = |entity_data, position, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, orientation,
            Entity.multi_count(entity_data),
            for_rigid_body
        ))
    )

## Creates a new reference frame component with the given position,
## orientation and scaling for a rigid body. The origin offset will be set
## to the center of mass.
for_scaled_rigid_body : Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
for_scaled_rigid_body = |position, orientation, scaling|
    new(position, orientation, scaling)

## Creates a new reference frame component with the given position,
## orientation and scaling for a rigid body. The origin offset will be set
## to the center of mass.
## Adds the component to the given entity's data.
add_for_scaled_rigid_body : Entity.Data, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_for_scaled_rigid_body = |entity_data, position, orientation, scaling|
    add(entity_data, for_scaled_rigid_body(position, orientation, scaling))

## Creates a new reference frame component with the given position,
## orientation and scaling for a rigid body. The origin offset will be set
## to the center of mass.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_rigid_body : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_rigid_body = |entity_data, position, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            position, orientation, scaling,
            Entity.multi_count(entity_data),
            for_scaled_rigid_body
        ))
    )

## Creates a new reference frame component with the given position for a
## rigid body with the identity orientation and scaling. The origin offset
## will be set to the center of mass.
for_unoriented_rigid_body : Point3.Point3 Binary64 -> ReferenceFrame
for_unoriented_rigid_body = |position|
    unoriented(position)

## Creates a new reference frame component with the given position for a
## rigid body with the identity orientation and scaling. The origin offset
## will be set to the center of mass.
## Adds the component to the given entity's data.
add_for_unoriented_rigid_body : Entity.Data, Point3.Point3 Binary64 -> Entity.Data
add_for_unoriented_rigid_body = |entity_data, position|
    add(entity_data, for_unoriented_rigid_body(position))

## Creates a new reference frame component with the given position for a
## rigid body with the identity orientation and scaling. The origin offset
## will be set to the center of mass.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_unoriented_rigid_body : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64) -> Result Entity.MultiData Str
add_multiple_for_unoriented_rigid_body = |entity_data, position|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            position,
            Entity.multi_count(entity_data),
            for_unoriented_rigid_body
        ))
    )

## Creates a new reference frame component with the given position and
## scaling for a rigid body with the identity orientation. The origin
## offset will be set to the center of mass.
for_scaled_unoriented_rigid_body : Point3.Point3 Binary64, F64 -> ReferenceFrame
for_scaled_unoriented_rigid_body = |position, scaling|
    unoriented_scaled(position, scaling)

## Creates a new reference frame component with the given position and
## scaling for a rigid body with the identity orientation. The origin
## offset will be set to the center of mass.
## Adds the component to the given entity's data.
add_for_scaled_unoriented_rigid_body : Entity.Data, Point3.Point3 Binary64, F64 -> Entity.Data
add_for_scaled_unoriented_rigid_body = |entity_data, position, scaling|
    add(entity_data, for_scaled_unoriented_rigid_body(position, scaling))

## Creates a new reference frame component with the given position and
## scaling for a rigid body with the identity orientation. The origin
## offset will be set to the center of mass.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_unoriented_rigid_body : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_unoriented_rigid_body = |entity_data, position, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, scaling,
            Entity.multi_count(entity_data),
            for_scaled_unoriented_rigid_body
        ))
    )

## Creates a new reference frame component with the given position for an
## entity whose orientation will be evolved analytically (and thus should
## not be initialised in this component).
for_driven_rotation : Point3.Point3 Binary64 -> ReferenceFrame
for_driven_rotation = |position|
    unoriented(position)

## Creates a new reference frame component with the given position for an
## entity whose orientation will be evolved analytically (and thus should
## not be initialised in this component).
## Adds the component to the given entity's data.
add_for_driven_rotation : Entity.Data, Point3.Point3 Binary64 -> Entity.Data
add_for_driven_rotation = |entity_data, position|
    add(entity_data, for_driven_rotation(position))

## Creates a new reference frame component with the given position for an
## entity whose orientation will be evolved analytically (and thus should
## not be initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_driven_rotation : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64) -> Result Entity.MultiData Str
add_multiple_for_driven_rotation = |entity_data, position|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            position,
            Entity.multi_count(entity_data),
            for_driven_rotation
        ))
    )

## Creates a new reference frame component with the given position and
## scaling for an entity whose orientation will be evolved analytically
## (and thus should not be initialised in this component).
for_scaled_driven_rotation : Point3.Point3 Binary64, F64 -> ReferenceFrame
for_scaled_driven_rotation = |position, scaling|
    unoriented_scaled(position, scaling)

## Creates a new reference frame component with the given position and
## scaling for an entity whose orientation will be evolved analytically
## (and thus should not be initialised in this component).
## Adds the component to the given entity's data.
add_for_scaled_driven_rotation : Entity.Data, Point3.Point3 Binary64, F64 -> Entity.Data
add_for_scaled_driven_rotation = |entity_data, position, scaling|
    add(entity_data, for_scaled_driven_rotation(position, scaling))

## Creates a new reference frame component with the given position and
## scaling for an entity whose orientation will be evolved analytically
## (and thus should not be initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_driven_rotation : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_driven_rotation = |entity_data, position, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, scaling,
            Entity.multi_count(entity_data),
            for_scaled_driven_rotation
        ))
    )

## Creates a new reference frame component with the given origin offset and
## position for an entity with no scaling whose orientation will be evolved
## analytically (and thus should not be initialised in this component).
for_driven_rotation_around_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64 -> ReferenceFrame
for_driven_rotation_around_offset_origin = |origin_offset, position|
    unoriented_with_offset_origin(origin_offset, position)

## Creates a new reference frame component with the given origin offset and
## position for an entity with no scaling whose orientation will be evolved
## analytically (and thus should not be initialised in this component).
## Adds the component to the given entity's data.
add_for_driven_rotation_around_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64 -> Entity.Data
add_for_driven_rotation_around_offset_origin = |entity_data, origin_offset, position|
    add(entity_data, for_driven_rotation_around_offset_origin(origin_offset, position))

## Creates a new reference frame component with the given origin offset and
## position for an entity with no scaling whose orientation will be evolved
## analytically (and thus should not be initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_driven_rotation_around_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64) -> Result Entity.MultiData Str
add_multiple_for_driven_rotation_around_offset_origin = |entity_data, origin_offset, position|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            origin_offset, position,
            Entity.multi_count(entity_data),
            for_driven_rotation_around_offset_origin
        ))
    )

## Creates a new reference frame component with the given origin offset,
## position and scaling for an entity whose orientation will be evolved
## analytically (and thus should not be initialised in this component).
for_scaled_driven_rotation_around_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64, F64 -> ReferenceFrame
for_scaled_driven_rotation_around_offset_origin = |origin_offset, position, scaling|
    unoriented_scaled_with_offset_origin(origin_offset, position, scaling)

## Creates a new reference frame component with the given origin offset,
## position and scaling for an entity whose orientation will be evolved
## analytically (and thus should not be initialised in this component).
## Adds the component to the given entity's data.
add_for_scaled_driven_rotation_around_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64, F64 -> Entity.Data
add_for_scaled_driven_rotation_around_offset_origin = |entity_data, origin_offset, position, scaling|
    add(entity_data, for_scaled_driven_rotation_around_offset_origin(origin_offset, position, scaling))

## Creates a new reference frame component with the given origin offset,
## position and scaling for an entity whose orientation will be evolved
## analytically (and thus should not be initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_driven_rotation_around_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_driven_rotation_around_offset_origin = |entity_data, origin_offset, position, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin_offset, position, scaling,
            Entity.multi_count(entity_data),
            for_scaled_driven_rotation_around_offset_origin
        ))
    )

## Creates a new reference frame component with the given orientation for
## an entity with no scaling whose trajectory will be evolved analytically
## (and whose position should thus not be initialised in this component).
for_driven_trajectory : UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
for_driven_trajectory = |orientation|
    unlocated(orientation)

## Creates a new reference frame component with the given orientation for
## an entity with no scaling whose trajectory will be evolved analytically
## (and whose position should thus not be initialised in this component).
## Adds the component to the given entity's data.
add_for_driven_trajectory : Entity.Data, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_for_driven_trajectory = |entity_data, orientation|
    add(entity_data, for_driven_trajectory(orientation))

## Creates a new reference frame component with the given orientation for
## an entity with no scaling whose trajectory will be evolved analytically
## (and whose position should thus not be initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_driven_trajectory : Entity.MultiData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_for_driven_trajectory = |entity_data, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            orientation,
            Entity.multi_count(entity_data),
            for_driven_trajectory
        ))
    )

## Creates a new reference frame component with the given orientation and
## scaling for an entity whose trajectory will be evolved analytically (and
## whose position should thus not be initialised in this component).
for_scaled_driven_trajectory : UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
for_scaled_driven_trajectory = |orientation, scaling|
    unlocated_scaled(orientation, scaling)

## Creates a new reference frame component with the given orientation and
## scaling for an entity whose trajectory will be evolved analytically (and
## whose position should thus not be initialised in this component).
## Adds the component to the given entity's data.
add_for_scaled_driven_trajectory : Entity.Data, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_for_scaled_driven_trajectory = |entity_data, orientation, scaling|
    add(entity_data, for_scaled_driven_trajectory(orientation, scaling))

## Creates a new reference frame component with the given orientation and
## scaling for an entity whose trajectory will be evolved analytically (and
## whose position should thus not be initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_driven_trajectory : Entity.MultiData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_driven_trajectory = |entity_data, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            orientation, scaling,
            Entity.multi_count(entity_data),
            for_scaled_driven_trajectory
        ))
    )

## Creates a new reference frame component with the given origin offset and
## orientation for an entity with no scaling whose trajectory will be
## evolved analytically (and whose position should thus not be initialised
## in this component).
for_driven_trajectory_with_offset_origin : Vector3.Vector3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
for_driven_trajectory_with_offset_origin = |origin_offset, orientation|
    for_scaled_driven_trajectory_with_offset_origin(origin_offset, orientation, 1.0)

## Creates a new reference frame component with the given origin offset and
## orientation for an entity with no scaling whose trajectory will be
## evolved analytically (and whose position should thus not be initialised
## in this component).
## Adds the component to the given entity's data.
add_for_driven_trajectory_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_for_driven_trajectory_with_offset_origin = |entity_data, origin_offset, orientation|
    add(entity_data, for_driven_trajectory_with_offset_origin(origin_offset, orientation))

## Creates a new reference frame component with the given origin offset and
## orientation for an entity with no scaling whose trajectory will be
## evolved analytically (and whose position should thus not be initialised
## in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_driven_trajectory_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_for_driven_trajectory_with_offset_origin = |entity_data, origin_offset, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            origin_offset, orientation,
            Entity.multi_count(entity_data),
            for_driven_trajectory_with_offset_origin
        ))
    )

## Creates a new reference frame component with the given origin offset,
## orientation and scaling for an entity whose trajectory will be evolved
## analytically (and whose position should thus not be initialised in this
## component).
for_scaled_driven_trajectory_with_offset_origin : Vector3.Vector3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
for_scaled_driven_trajectory_with_offset_origin = |origin_offset, orientation, scaling|
    scaled_with_offset_origin(origin_offset, Point3.origin, orientation, scaling)

## Creates a new reference frame component with the given origin offset,
## orientation and scaling for an entity whose trajectory will be evolved
## analytically (and whose position should thus not be initialised in this
## component).
## Adds the component to the given entity's data.
add_for_scaled_driven_trajectory_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_for_scaled_driven_trajectory_with_offset_origin = |entity_data, origin_offset, orientation, scaling|
    add(entity_data, for_scaled_driven_trajectory_with_offset_origin(origin_offset, orientation, scaling))

## Creates a new reference frame component with the given origin offset,
## orientation and scaling for an entity whose trajectory will be evolved
## analytically (and whose position should thus not be initialised in this
## component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_driven_trajectory_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_driven_trajectory_with_offset_origin = |entity_data, origin_offset, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin_offset, orientation, scaling,
            Entity.multi_count(entity_data),
            for_scaled_driven_trajectory_with_offset_origin
        ))
    )

## Creates a new reference frame component for an entity with no scaling
## whose trajectory and orientation will be evolved analytically (and whose
## position and orientation should thus not be initialised in this
## component).
for_driven_trajectory_and_rotation : {} -> ReferenceFrame
for_driven_trajectory_and_rotation = |{}|
    {
        origin_offset: Vector3.zero,
        position: Point3.origin,
        orientation: UnitQuaternion.identity,
        scaling: 1.0,
    }

## Creates a new reference frame component for an entity with no scaling
## whose trajectory and orientation will be evolved analytically (and whose
## position and orientation should thus not be initialised in this
## component).
## Adds the component to the given entity's data.
add_for_driven_trajectory_and_rotation : Entity.Data -> Entity.Data
add_for_driven_trajectory_and_rotation = |entity_data|
    add(entity_data, for_driven_trajectory_and_rotation({}))

## Creates a new reference frame component for an entity with no scaling
## whose trajectory and orientation will be evolved analytically (and whose
## position and orientation should thus not be initialised in this
## component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_driven_trajectory_and_rotation : Entity.MultiData -> Entity.MultiData
add_multiple_for_driven_trajectory_and_rotation = |entity_data|
    res = add_multiple(
        entity_data,
        Same(for_driven_trajectory_and_rotation({}))
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in ReferenceFrame.add_multiple_for_driven_trajectory_and_rotation: ${Inspect.to_str(err)}"

## Creates a new reference frame component for an entity with the given
## scaling whose trajectory and orientation will be evolved analytically
## (and whose position and orientation should thus not be initialised in
## this component).
for_scaled_driven_trajectory_and_rotation : F64 -> ReferenceFrame
for_scaled_driven_trajectory_and_rotation = |scaling|
    scaled(scaling)

## Creates a new reference frame component for an entity with the given
## scaling whose trajectory and orientation will be evolved analytically
## (and whose position and orientation should thus not be initialised in
## this component).
## Adds the component to the given entity's data.
add_for_scaled_driven_trajectory_and_rotation : Entity.Data, F64 -> Entity.Data
add_for_scaled_driven_trajectory_and_rotation = |entity_data, scaling|
    add(entity_data, for_scaled_driven_trajectory_and_rotation(scaling))

## Creates a new reference frame component for an entity with the given
## scaling whose trajectory and orientation will be evolved analytically
## (and whose position and orientation should thus not be initialised in
## this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_driven_trajectory_and_rotation : Entity.MultiData, Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_driven_trajectory_and_rotation = |entity_data, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            scaling,
            Entity.multi_count(entity_data),
            for_scaled_driven_trajectory_and_rotation
        ))
    )

## Creates a new reference frame component with the given origin offset for
## an entity with no scaling whose trajectory and orientation will be
## evolved analytically (and whose position and orientation should thus not
## be initialised in this component).
for_driven_trajectory_and_rotation_with_offset_origin : Vector3.Vector3 Binary64 -> ReferenceFrame
for_driven_trajectory_and_rotation_with_offset_origin = |origin_offset|
    for_scaled_driven_trajectory_and_rotation_with_offset_origin(origin_offset, 1.0)

## Creates a new reference frame component with the given origin offset for
## an entity with no scaling whose trajectory and orientation will be
## evolved analytically (and whose position and orientation should thus not
## be initialised in this component).
## Adds the component to the given entity's data.
add_for_driven_trajectory_and_rotation_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64 -> Entity.Data
add_for_driven_trajectory_and_rotation_with_offset_origin = |entity_data, origin_offset|
    add(entity_data, for_driven_trajectory_and_rotation_with_offset_origin(origin_offset))

## Creates a new reference frame component with the given origin offset for
## an entity with no scaling whose trajectory and orientation will be
## evolved analytically (and whose position and orientation should thus not
## be initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_driven_trajectory_and_rotation_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64) -> Result Entity.MultiData Str
add_multiple_for_driven_trajectory_and_rotation_with_offset_origin = |entity_data, origin_offset|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            origin_offset,
            Entity.multi_count(entity_data),
            for_driven_trajectory_and_rotation_with_offset_origin
        ))
    )

## Creates a new reference frame component with the given origin offset and
## scaling for an entity whose trajectory and orientation will be evolved
## analytically (and whose position and orientation should thus not be
## initialised in this component).
for_scaled_driven_trajectory_and_rotation_with_offset_origin : Vector3.Vector3 Binary64, F64 -> ReferenceFrame
for_scaled_driven_trajectory_and_rotation_with_offset_origin = |origin_offset, scaling|
    for_scaled_driven_trajectory_with_offset_origin(origin_offset, UnitQuaternion.identity, scaling)

## Creates a new reference frame component with the given origin offset and
## scaling for an entity whose trajectory and orientation will be evolved
## analytically (and whose position and orientation should thus not be
## initialised in this component).
## Adds the component to the given entity's data.
add_for_scaled_driven_trajectory_and_rotation_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, F64 -> Entity.Data
add_for_scaled_driven_trajectory_and_rotation_with_offset_origin = |entity_data, origin_offset, scaling|
    add(entity_data, for_scaled_driven_trajectory_and_rotation_with_offset_origin(origin_offset, scaling))

## Creates a new reference frame component with the given origin offset and
## scaling for an entity whose trajectory and orientation will be evolved
## analytically (and whose position and orientation should thus not be
## initialised in this component).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_for_scaled_driven_trajectory_and_rotation_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_for_scaled_driven_trajectory_and_rotation_with_offset_origin = |entity_data, origin_offset, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            origin_offset, scaling,
            Entity.multi_count(entity_data),
            for_scaled_driven_trajectory_and_rotation_with_offset_origin
        ))
    )

## Adds a value of the [ReferenceFrame] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ReferenceFrame -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ReferenceFrame] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ReferenceFrame) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ReferenceFrame.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ReferenceFrame -> List U8
write_packet = |bytes, val|
    type_id = 32432739310383407
    size = 88
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ReferenceFrame -> List U8
write_multi_packet = |bytes, vals|
    type_id = 32432739310383407
    size = 88
    alignment = 8
    count = List.len(vals)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    vals
    |> List.walk(
        bytes_with_header,
        |bts, value| bts |> write_bytes(value),
    )

## Serializes a value of [ReferenceFrame] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ReferenceFrame -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(88)
    |> Vector3.write_bytes_64(value.origin_offset)
    |> Point3.write_bytes_64(value.position)
    |> UnitQuaternion.write_bytes_64(value.orientation)
    |> Builtin.write_bytes_f64(value.scaling)

## Deserializes a value of [ReferenceFrame] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ReferenceFrame _
from_bytes = |bytes|
    Ok(
        {
            origin_offset: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            position: bytes |> List.sublist({ start: 24, len: 24 }) |> Point3.from_bytes_64?,
            orientation: bytes |> List.sublist({ start: 48, len: 32 }) |> UnitQuaternion.from_bytes_64?,
            scaling: bytes |> List.sublist({ start: 80, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 88 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
