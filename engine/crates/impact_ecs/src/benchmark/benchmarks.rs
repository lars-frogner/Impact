//! Benchmarks.

use crate::{Component, world::World};
use bytemuck::{Pod, Zeroable};
use impact_id::{EntityID, EntityIDManager};

pub mod entity;
pub mod query;

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

fn populate_world(world: &mut World) -> Vec<EntityID> {
    let mut id_manager = EntityIDManager::new();
    let mut entities = Vec::new();

    let mut add_ids = |count: usize| {
        let ids = id_manager.provide_id_vec(count);
        entities.extend_from_slice(&ids);
        ids
    };

    world.create_entity(add_ids(1)[0], &ZeroSized).unwrap();
    world.create_entities(add_ids(5), &[F32_TUPLE; 5]).unwrap();
    world.create_entities(add_ids(3), &[F64_TUPLE; 3]).unwrap();
    world.create_entities(add_ids(2), &[F32_TRIPLE; 2]).unwrap();
    world.create_entities(add_ids(6), &[F64_TRIPLE; 6]).unwrap();
    world
        .create_entities(add_ids(2), (&[F32_TUPLE; 2], &[F64_TUPLE; 2]))
        .unwrap();
    world
        .create_entities(add_ids(6), (&[F32_TRIPLE; 6], &[F64_TRIPLE; 6]))
        .unwrap();
    world
        .create_entities(
            add_ids(2),
            (&[F32_TUPLE; 2], &[F64_TUPLE; 2], &[F32_TRIPLE; 2]),
        )
        .unwrap();
    world
        .create_entities(
            add_ids(1),
            (&[F32_TUPLE; 1], &[F64_TUPLE; 1], &[F64_TRIPLE; 1]),
        )
        .unwrap();
    world
        .create_entities(
            add_ids(4),
            (
                &[F32_TUPLE; 4],
                &[F64_TUPLE; 4],
                &[F32_TRIPLE; 4],
                &[F64_TRIPLE; 4],
            ),
        )
        .unwrap();

    entities
}
