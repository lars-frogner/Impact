use super::{
    F32_TRIPLE, F32_TUPLE, F32TripleComp, F64_TRIPLE, F64_TUPLE, ZeroSized, populate_world,
};
use crate::world::World;
use impact_profiling::Profiler;

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
    let entity_id = world
        .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
        .unwrap();
    profiler.profile(&mut || {
        let entry = world.entity(entity_id);
        entry.n_components()
    });
}

pub fn get_one_of_many_different_entities(profiler: impl Profiler) {
    let mut world = World::new();
    let entities = populate_world(&mut world);
    let entity = entities[21];
    profiler.profile(&mut || {
        let entry = world.entity(entity);
        entry.n_components()
    });
}

pub fn get_component_of_only_entity(profiler: impl Profiler) {
    let mut world = World::new();
    let entity_id = world
        .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
        .unwrap();
    profiler.profile(&mut || {
        let entry = world.entity(entity_id);
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
        let entry = world.entity(entity);
        let comp_entry = entry.component::<F32TripleComp>();
        let comp = comp_entry.access();
        *comp
    });
}

pub fn modify_component_of_only_entity(profiler: impl Profiler) {
    let mut world = World::new();
    let entity_id = world
        .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
        .unwrap();
    profiler.profile(&mut || {
        let entry = world.entity(entity_id);
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
        let entry = world.entity(entity);
        let mut comp_entry = entry.component_mut::<F32TripleComp>();
        let comp = comp_entry.access();
        comp.1 = 42.0;
        *comp
    });
}
