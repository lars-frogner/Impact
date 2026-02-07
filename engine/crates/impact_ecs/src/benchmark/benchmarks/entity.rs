use super::{
    F32_TRIPLE, F32_TUPLE, F32TripleComp, F64_TRIPLE, F64_TUPLE, ZeroSized, populate_world,
};
use crate::world::World;
use impact_id::EntityID;
use impact_profiling::benchmark::Benchmarker;

pub fn create_single_entity_single_comp(benchmarker: impl Benchmarker) {
    benchmarker.benchmark(&mut || {
        let mut world = World::new();
        world
            .create_entity(EntityID::from_u64(0), &F32_TRIPLE)
            .unwrap();
        world
    });
}

pub fn create_single_entity_multiple_comps(benchmarker: impl Benchmarker) {
    benchmarker.benchmark(&mut || {
        let mut world = World::new();
        world
            .create_entity(
                EntityID::from_u64(0),
                (&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE),
            )
            .unwrap();
        world
    });
}

pub fn create_multiple_identical_entities(benchmarker: impl Benchmarker) {
    const COUNT: usize = 10;
    let entity_ids = Vec::from_iter((0..COUNT as u64).map(EntityID::from_u64));

    benchmarker.benchmark(&mut || {
        let mut world = World::new();
        world
            .create_entities(
                entity_ids.iter().copied(),
                (
                    &[F32_TUPLE; COUNT],
                    &[F64_TUPLE; COUNT],
                    &[F32_TRIPLE; COUNT],
                    &[F64_TRIPLE; COUNT],
                ),
            )
            .unwrap();
        world
    });
}

pub fn create_multiple_different_entities(benchmarker: impl Benchmarker) {
    benchmarker.benchmark(&mut || {
        let mut world = World::new();
        let mut id = 0;
        world
            .create_entity(EntityID::from_u64(id), &ZeroSized)
            .unwrap();
        id += 1;
        world
            .create_entity(EntityID::from_u64(id), &F32_TUPLE)
            .unwrap();
        id += 1;
        world
            .create_entity(EntityID::from_u64(id), &F64_TUPLE)
            .unwrap();
        id += 1;
        world
            .create_entity(EntityID::from_u64(id), &F32_TRIPLE)
            .unwrap();
        id += 1;
        world
            .create_entity(EntityID::from_u64(id), &F64_TRIPLE)
            .unwrap();
        id += 1;
        world
            .create_entity(EntityID::from_u64(id), (&F32_TUPLE, &F64_TUPLE))
            .unwrap();
        id += 1;
        world
            .create_entity(EntityID::from_u64(id), (&F32_TRIPLE, &F64_TRIPLE))
            .unwrap();
        id += 1;
        world
            .create_entity(
                EntityID::from_u64(id),
                (&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE),
            )
            .unwrap();
        id += 1;
        world
            .create_entity(
                EntityID::from_u64(id),
                (&F32_TUPLE, &F64_TUPLE, &F64_TRIPLE),
            )
            .unwrap();
        id += 1;
        world
            .create_entity(
                EntityID::from_u64(id),
                (&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE),
            )
            .unwrap();
        world
    });
}

pub fn get_only_entity(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    let entity_id = EntityID::from_u64(0);
    world
        .create_entity(
            entity_id,
            (&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE),
        )
        .unwrap();

    benchmarker.benchmark(&mut || {
        let entry = world.entity(entity_id);
        entry.n_components()
    });
}

pub fn get_one_of_many_different_entities(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    let entities = populate_world(&mut world);
    let entity = entities[21];

    benchmarker.benchmark(&mut || {
        let entry = world.entity(entity);
        entry.n_components()
    });
}

pub fn get_component_of_only_entity(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    let entity_id = EntityID::from_u64(0);
    world
        .create_entity(
            entity_id,
            (&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE),
        )
        .unwrap();

    benchmarker.benchmark(&mut || {
        let entry = world.entity(entity_id);
        let comp_entry = entry.component::<F32TripleComp>();
        let comp = comp_entry.access();
        *comp
    });
}

pub fn get_component_of_one_of_many_different_entities(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    let entities = populate_world(&mut world);
    let entity = entities[21];

    benchmarker.benchmark(&mut || {
        let entry = world.entity(entity);
        let comp_entry = entry.component::<F32TripleComp>();
        let comp = comp_entry.access();
        *comp
    });
}

pub fn modify_component_of_only_entity(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    let entity_id = EntityID::from_u64(0);
    world
        .create_entity(
            entity_id,
            (&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE),
        )
        .unwrap();

    benchmarker.benchmark(&mut || {
        let entry = world.entity(entity_id);
        let mut comp_entry = entry.component_mut::<F32TripleComp>();
        let comp = comp_entry.access();
        comp.1 = 42.0;
        *comp
    });
}

pub fn modify_component_of_one_of_many_different_entities(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    let entities = populate_world(&mut world);
    let entity = entities[21];

    benchmarker.benchmark(&mut || {
        let entry = world.entity(entity);
        let mut comp_entry = entry.component_mut::<F32TripleComp>();
        let comp = comp_entry.access();
        comp.1 = 42.0;
        *comp
    });
}
