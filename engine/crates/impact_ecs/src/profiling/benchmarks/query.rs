use super::{F32_TRIPLE, F32_TUPLE, F32TripleComp, F64_TRIPLE, F64_TUPLE, populate_world};
use crate::{
    profiling::benchmarks::{F32TupleComp, F64TripleComp, F64TupleComp},
    world::World,
};
use impact_ecs_macros::query;
use impact_profiling::Profiler;

pub fn query_single_comp_single_entity(profiler: impl Profiler) {
    let mut world = World::default();
    world.create_entity(&F32_TRIPLE).unwrap();
    profiler.profile(&mut || {
        let mut copy = F32_TRIPLE;
        query!(&world, |comp: &F32TripleComp| {
            copy = *comp;
        });
        copy
    });
}

pub fn query_single_comp_multiple_identical_entities(profiler: impl Profiler) {
    let mut world = World::default();
    world.create_entities(&[F32_TRIPLE; 10]).unwrap();
    profiler.profile(&mut || {
        let mut copy = F32_TRIPLE;
        query!(&world, |comp: &F32TripleComp| {
            copy = *comp;
        });
        copy
    });
}

pub fn query_multiple_comps_single_entity(profiler: impl Profiler) {
    let mut world = World::default();
    world
        .create_entity((&F32_TUPLE, &F64_TUPLE, &F32_TRIPLE, &F64_TRIPLE))
        .unwrap();
    profiler.profile(&mut || {
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

pub fn query_multiple_comps_multiple_identical_entities(profiler: impl Profiler) {
    let mut world = World::default();
    world
        .create_entities((
            &[F32_TUPLE; 10],
            &[F64_TUPLE; 10],
            &[F32_TRIPLE; 10],
            &[F64_TRIPLE; 10],
        ))
        .unwrap();
    profiler.profile(&mut || {
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

pub fn query_single_comp_multiple_different_entities(profiler: impl Profiler) {
    let mut world = World::default();
    populate_world(&mut world);
    profiler.profile(&mut || {
        let mut copy = F32_TRIPLE;
        query!(&world, |comp: &F32TripleComp| {
            copy = *comp;
        });
        copy
    });
}

pub fn query_multiple_comps_multiple_different_entities(profiler: impl Profiler) {
    let mut world = World::default();
    populate_world(&mut world);
    profiler.profile(&mut || {
        let mut copy = (F32_TUPLE, F64_TUPLE);
        query!(&world, |f32_tuple: &F32TupleComp,
                        f64_tuple: &F64TupleComp| {
            copy = (*f32_tuple, *f64_tuple);
        });
        copy
    });
}
