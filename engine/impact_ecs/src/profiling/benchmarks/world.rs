use crate::{
    Component,
    world::{Entity, World},
};
use bytemuck::{Pod, Zeroable};
use impact_profiling::Profiler;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
struct ZeroSized;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
struct F32TupleComp(f32, f32);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
struct F64TupleComp(f64, f64);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
struct F32TripleComp(f32, f32, f32);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
struct F64TripleComp(f64, f64, f64);

const F32_TUPLE: F32TupleComp = F32TupleComp(0.0, 1.0);
const F64_TUPLE: F64TupleComp = F64TupleComp(0.0, 1.0);
const F32_TRIPLE: F32TripleComp = F32TripleComp(0.0, 1.0, 2.0);
const F64_TRIPLE: F64TripleComp = F64TripleComp(0.0, 1.0, 2.0);

pub fn create_single_entity_single_comp(profiler: impl Profiler) {
    profiler.profile(&mut || {
        let mut world = World::new();
        world.create_entity(&F32_TRIPLE).unwrap();
        world
    });
}

pub fn create_single_entity_multiple_comps(profiler: impl Profiler) {
    profiler.profile(&mut || {
        let mut world = World::new();
        world
            .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
            .unwrap();
        world
    });
}

pub fn create_multiple_identical_entities(profiler: impl Profiler) {
    profiler.profile(&mut || {
        let mut world = World::new();
        world
            .create_entities((
                &[F32_TUPLE; 10],
                &[F64_TUPLE; 10],
                &[F32_TRIPLE; 10],
                &[F64_TRIPLE; 10],
            ))
            .unwrap();
        world
    });
}

pub fn create_multiple_different_entities(profiler: impl Profiler) {
    profiler.profile(&mut || {
        let mut world = World::new();
        world.create_entity(&ZeroSized).unwrap();
        world.create_entity(&F32_TUPLE).unwrap();
        world.create_entity(&F64_TUPLE).unwrap();
        world.create_entity(&F32_TRIPLE).unwrap();
        world.create_entity(&F64_TRIPLE).unwrap();
        world.create_entity((&F32_TUPLE, &F64_TUPLE)).unwrap();
        world.create_entity((&F32_TRIPLE, &F64_TRIPLE)).unwrap();
        world
            .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE))
            .unwrap();
        world
            .create_entity((&F32_TUPLE, &F64_TUPLE, &F64_TRIPLE))
            .unwrap();
        world
            .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
            .unwrap();
        world
    });
}

pub fn get_only_entity(profiler: impl Profiler) {
    let mut world = World::new();
    let entity = world
        .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
        .unwrap();
    profiler.profile(&mut || {
        let entry = world.entity(&entity);
        entry.n_components()
    });
}

pub fn get_one_of_many_different_entities(profiler: impl Profiler) {
    let mut world = World::new();
    let entities = populate_world(&mut world);
    let entity = entities[21];
    profiler.profile(&mut || {
        let entry = world.entity(&entity);
        entry.n_components()
    });
}

pub fn get_component_of_only_entity(profiler: impl Profiler) {
    let mut world = World::new();
    let entity = world
        .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
        .unwrap();
    profiler.profile(&mut || {
        let entry = world.entity(&entity);
        let comp_entry = entry.component::<F32TripleComp>();
        let comp = comp_entry.access();
        *comp
    });
}

pub fn get_component_of_one_of_many_different_entities(profiler: impl Profiler) {
    let mut world = World::new();
    let entities = populate_world(&mut world);
    let entity = entities[21];
    profiler.profile(&mut || {
        let entry = world.entity(&entity);
        let comp_entry = entry.component::<F32TripleComp>();
        let comp = comp_entry.access();
        *comp
    });
}

pub fn modify_component_of_only_entity(profiler: impl Profiler) {
    let mut world = World::new();
    let entity = world
        .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
        .unwrap();
    profiler.profile(&mut || {
        let entry = world.entity(&entity);
        let mut comp_entry = entry.component_mut::<F32TripleComp>();
        let comp = comp_entry.access();
        comp.1 = 42.0;
        *comp
    });
}

pub fn modify_component_of_one_of_many_different_entities(profiler: impl Profiler) {
    let mut world = World::new();
    let entities = populate_world(&mut world);
    let entity = entities[21];
    profiler.profile(&mut || {
        let entry = world.entity(&entity);
        let mut comp_entry = entry.component_mut::<F32TripleComp>();
        let comp = comp_entry.access();
        comp.1 = 42.0;
        *comp
    });
}

fn populate_world(world: &mut World) -> Vec<Entity> {
    let mut entities = Vec::new();
    entities.push(world.create_entity(&ZeroSized).unwrap());
    entities.extend(world.create_entities(&[F32_TUPLE; 5]).unwrap());
    entities.extend(world.create_entities(&[F64_TUPLE; 3]).unwrap());
    entities.extend(world.create_entities(&[F32_TRIPLE; 2]).unwrap());
    entities.extend(world.create_entities(&[F64_TRIPLE; 6]).unwrap());
    entities.extend(
        world
            .create_entities((&[F32_TUPLE; 2], &[F64_TUPLE; 2]))
            .unwrap(),
    );
    entities.extend(
        world
            .create_entities((&[F32_TRIPLE; 6], &[F64_TRIPLE; 6]))
            .unwrap(),
    );
    entities.extend(
        world
            .create_entities((&[F32_TUPLE; 2], &[F64_TUPLE; 2], &[F32_TRIPLE; 2]))
            .unwrap(),
    );
    entities.extend(
        world
            .create_entities((&[F32_TUPLE; 1], &[F64_TUPLE; 1], &[F64_TRIPLE; 1]))
            .unwrap(),
    );
    entities.extend(
        world
            .create_entities((
                &[F32_TUPLE; 4],
                &[F64_TUPLE; 4],
                &[F32_TRIPLE; 4],
                &[F64_TRIPLE; 4],
            ))
            .unwrap(),
    );
    entities
}
