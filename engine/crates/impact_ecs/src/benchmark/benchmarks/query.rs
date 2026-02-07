use super::{F32_TRIPLE, F32_TUPLE, F32TripleComp, F64_TRIPLE, F64_TUPLE, populate_world};
use crate::{
    benchmark::benchmarks::{F32TupleComp, F64TripleComp, F64TupleComp},
    world::World,
};
use impact_ecs_macros::query;
use impact_id::EntityID;
use impact_profiling::benchmark::Benchmarker;

pub fn query_single_comp_single_entity(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(0), &F32_TRIPLE)
        .unwrap();

    benchmarker.benchmark(&mut || {
        let mut copy = F32_TRIPLE;
        query!(&world, |comp: &F32TripleComp| {
            copy = *comp;
        });
        copy
    });
}

pub fn query_single_comp_multiple_identical_entities(benchmarker: impl Benchmarker) {
    const COUNT: usize = 10;
    let entity_ids = Vec::from_iter((0..COUNT as u64).map(EntityID::from_u64));

    let mut world = World::new();
    world
        .create_entities(entity_ids, &[F32_TRIPLE; COUNT])
        .unwrap();

    benchmarker.benchmark(&mut || {
        let mut copy = F32_TRIPLE;
        query!(&world, |comp: &F32TripleComp| {
            copy = *comp;
        });
        copy
    });
}

pub fn query_multiple_comps_single_entity(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    world
        .create_entity(
            EntityID::from_u64(0),
            (&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE),
        )
        .unwrap();

    benchmarker.benchmark(&mut || {
        let mut copy = (F32_TUPLE, F64_TUPLE, F32_TRIPLE, F64_TRIPLE);
        query!(&world, |f32_tuple: &F32TupleComp,
                        f64_tuple: &F64TupleComp,
                        f32_triple: &F32TripleComp,
                        f64_triple: &F64TripleComp| {
            copy = (*f32_tuple, *f64_tuple, *f32_triple, *f64_triple);
        });
        copy
    });
}

pub fn query_multiple_comps_multiple_identical_entities(benchmarker: impl Benchmarker) {
    const COUNT: usize = 10;
    let entity_ids = Vec::from_iter((0..COUNT as u64).map(EntityID::from_u64));

    let mut world = World::new();
    world
        .create_entities(
            entity_ids,
            (
                &[F32_TUPLE; COUNT],
                &[F64_TUPLE; COUNT],
                &[F32_TRIPLE; COUNT],
                &[F64_TRIPLE; COUNT],
            ),
        )
        .unwrap();

    benchmarker.benchmark(&mut || {
        let mut copy = (F32_TUPLE, F64_TUPLE, F32_TRIPLE, F64_TRIPLE);
        query!(&world, |f32_tuple: &F32TupleComp,
                        f64_tuple: &F64TupleComp,
                        f32_triple: &F32TripleComp,
                        f64_triple: &F64TripleComp| {
            copy = (*f32_tuple, *f64_tuple, *f32_triple, *f64_triple);
        });
        copy
    });
}

pub fn query_single_comp_multiple_different_entities(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    populate_world(&mut world);
    benchmarker.benchmark(&mut || {
        let mut copy = F32_TRIPLE;
        query!(&world, |comp: &F32TripleComp| {
            copy = *comp;
        });
        copy
    });
}

pub fn query_multiple_comps_multiple_different_entities(benchmarker: impl Benchmarker) {
    let mut world = World::new();
    populate_world(&mut world);
    benchmarker.benchmark(&mut || {
        let mut copy = (F32_TUPLE, F64_TUPLE);
        query!(&world, |f32_tuple: &F32TupleComp,
                        f64_tuple: &F64TupleComp| {
            copy = (*f32_tuple, *f64_tuple);
        });
        copy
    });
}
