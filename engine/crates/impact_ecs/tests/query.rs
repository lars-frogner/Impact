//! Tests for the [`query`] macro.

use bytemuck::{Pod, Zeroable};
use impact_alloc::Global;
use impact_containers::HashSet;
use impact_ecs::{Component, query, world::World};
use impact_id::EntityID;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
struct Marked;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
struct Byte(u8);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
struct Position(f32, f32, f32);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
struct Rectangle {
    center: [f32; 2],
    dimensions: [f32; 2],
}

type LikeByte = Byte;

const BYTE: Byte = Byte(7);
const BYTE2: Byte = Byte(55);
const POS: Position = Position(1.5, -7.7, 0.1);
const POS2: Position = Position(0.0, 1e-5, 0.001);
const RECT: Rectangle = Rectangle {
    center: [2.5, 2.0],
    dimensions: [12.3, 8.9],
};

/// These query invocations should all compile successfully.
#[allow(dead_code, clippy::unnecessary_mut_passed)]
fn test_valid_query_inputs() {
    let mut world = World::new();

    query!(world, |_byte: &Byte| {});

    query!(&world, |_byte: &mut Byte| {});

    query!(world, |_pos: &mut Position, _byte: &Byte| {});

    query!(&mut world, |_byte: &mut Byte, _pos: &mut Position| {});

    query!(world, |_byte: &Byte,
                   _rect: &mut Rectangle,
                   _pos: &Position| {});

    query!(world, |_byte: &mut Byte,
                   _pos: &mut Position,
                   _rect: &mut Rectangle| {});

    query!(world, |_byte: &Byte| {}, []);

    query!(world, |_byte: &Byte| {}, [Position]);

    query!(world, |_byte: &Byte| {}, [Position, Rectangle]);

    query!(world, |_pos: &Position, _byte: &mut Byte| {}, [Rectangle]);

    query!(world, |_byte: &Byte| {}, ![]);

    query!(world, |_byte: &Byte| {}, ![Position]);

    query!(world, |_pos: &Position| {}, ![LikeByte]);

    query!(world, |_byte: &Byte| {}, ![Position, Rectangle]);

    query!(world, |_pos: &Position, _byte: &mut Byte| {}, ![Rectangle]);

    query!(world, |_byte: &Byte| {}, [], ![]);

    query!(world, |_byte: &Byte| {}, ![], []);

    query!(world, |_byte: &Byte| {}, [Position], ![Rectangle]);

    query!(world, |_byte: &Byte| {}, ![Position], [Rectangle]);

    query!(world, |_byte: &Byte| {}, [Position, Rectangle], ![Marked]);

    query!(world, |_byte: &Byte| {}, ![Position, Rectangle], [Marked]);

    // The macro accepts this because it does not know they are
    // the same type, but the result is just that there are no
    // matches
    query!(world, |_byte: &LikeByte| {}, ![Byte]);
    query!(world, |_pos: &Position| {}, ![Byte, LikeByte]);
    query!(world, |_pos: &Position| {}, [Byte], ![LikeByte]);

    // This compiles but panics at runtime
    query!(world, |_byte: &Byte, _likebyte: &LikeByte| {}, []);
    query!(world, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
#[should_panic]
fn querying_aliased_comps_fails_1() {
    let world = World::new();
    query!(world, |_byte: &Byte, _likebyte: &LikeByte| {});
}

#[test]
#[should_panic]
fn querying_aliased_comps_fails_2() {
    let world = World::new();
    query!(world, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
fn single_entity_single_comp_read_works() {
    let mut world = World::new();
    world.create_entity(EntityID::from_u64(1), &BYTE).unwrap();

    let mut count = 0;
    query!(world, |byte: &Byte| {
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn single_entity_two_of_two_matching_comp_read_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &POS))
        .unwrap();

    let mut count = 0;
    query!(world, |byte: &Byte, pos: &Position| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn single_entity_one_of_two_matching_comp_read_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &POS))
        .unwrap();

    let mut count = 0;
    query!(world, |pos: &Position| {
        assert_eq!(pos, &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn single_entity_single_comp_write_works() {
    let mut world = World::new();
    world.create_entity(EntityID::from_u64(1), &BYTE).unwrap();

    query!(world, |byte: &mut Byte| {
        assert_eq!(byte, &BYTE);
        *byte = BYTE2;
    });
    query!(world, |byte: &Byte| {
        assert_eq!(byte, &BYTE2);
    });
}

#[test]
fn single_entity_two_of_two_matching_comp_write_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &POS))
        .unwrap();

    query!(world, |pos: &mut Position, byte: &mut Byte| {
        assert_eq!(pos, &POS);
        assert_eq!(byte, &BYTE);
        *pos = POS2;
        *byte = BYTE2;
    });
    query!(world, |byte: &Byte, pos: &Position| {
        assert_eq!(pos, &POS2);
        assert_eq!(byte, &BYTE2);
    });
}

#[test]
fn single_entity_two_of_two_matching_comp_mixed_write_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &POS))
        .unwrap();

    query!(world, |pos: &mut Position, byte: &Byte| {
        assert_eq!(pos, &POS);
        assert_eq!(byte, &BYTE);
        pos.1 = f32::from(byte.0);
    });
    query!(world, |byte: &Byte, pos: &Position| {
        assert_eq!(pos.1, f32::from(byte.0));
    });
}

#[test]
fn two_equal_entities_single_comp_read_works() {
    let mut world = World::new();
    world
        .create_entities([1, 2].map(EntityID::from_u64), &[BYTE, BYTE])
        .unwrap();

    let mut count = 0;
    query!(world, |byte: &Byte| {
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn one_of_two_matching_single_comp_entities_works() {
    let mut world = World::new();
    world.create_entity(EntityID::from_u64(1), &BYTE).unwrap();
    world.create_entity(EntityID::from_u64(2), &POS).unwrap();

    let mut count = 0;
    query!(world, |pos: &Position| {
        assert_eq!(pos, &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn ono_of_two_matching_two_comp_entities_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&POS, &BYTE))
        .unwrap();

    let mut count = 0;
    query!(world, |rect: &Rectangle, byte: &Byte| {
        assert_eq!(rect, &RECT);
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn two_of_two_partially_matching_two_comp_entities_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&POS, &BYTE))
        .unwrap();

    let mut count = 0;
    query!(world, |byte: &Byte| {
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn zero_of_two_matching_two_comp_entities_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&POS, &BYTE))
        .unwrap();

    let mut count = 0;
    query!(world, |_pos: &Position, _rect: &Rectangle| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn one_additional_required_comp_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&POS, &BYTE))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&BYTE, &Marked))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(3), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(4), (&Marked, &BYTE, &POS))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(5), (&Marked, &POS))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        [Marked]
    );
    assert_eq!(count, 2);
}

#[test]
fn two_additional_required_comps_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT, &POS))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&Marked, &BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(3), (&POS, &Marked, &BYTE))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(4), (&POS, &RECT, &Marked, &BYTE))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        [Marked, Position]
    );
    assert_eq!(count, 2);
}

#[test]
fn excluding_one_comp_of_two_comp_entity_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        ![Rectangle]
    );
    assert_eq!(count, 0);
}

#[test]
fn excluding_one_aliased_comp_of_two_comp_entity_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_rect: &Rectangle| {
            count += 1;
        },
        ![LikeByte]
    );
    assert_eq!(count, 0);
}

#[test]
fn excluding_comp_that_is_alias_of_queried_comp_works() {
    let mut world = World::new();
    world.create_entity(EntityID::from_u64(1), &BYTE).unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        ![LikeByte]
    );
    assert_eq!(count, 0);
}

#[test]
fn excluding_one_of_two_two_comp_entities_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&POS, &BYTE))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        ![Rectangle]
    );
    assert_eq!(count, 1);
}

#[test]
fn excluding_one_of_a_two_and_three_comp_entity_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&POS, &BYTE, &RECT))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        ![Position]
    );
    assert_eq!(count, 1);
}

#[test]
fn excluding_both_of_a_two_and_three_comp_entity_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&POS, &BYTE, &RECT))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        ![Rectangle]
    );
    assert_eq!(count, 0);
}

#[test]
fn excluding_one_comp_of_three_comp_entity_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT, &POS))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte, _rect: &Rectangle| {
            count += 1;
        },
        ![Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn excluding_two_comps_of_three_comp_entity_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT, &POS))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_rect: &Rectangle| {
            count += 1;
        },
        ![Position, Byte]
    );
    assert_eq!(count, 0);
}

#[test]
fn excluding_a_comp_each_of_two_two_comp_entities_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&BYTE, &POS))
        .unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        ![Position, Rectangle]
    );
    assert_eq!(count, 0);
}

#[test]
fn excluding_two_of_three_entities_with_two_disallowed_comps_works() {
    let mut world = World::new();
    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();
    world
        .create_entity(EntityID::from_u64(2), (&BYTE, &POS))
        .unwrap();
    world.create_entity(EntityID::from_u64(3), &BYTE).unwrap();

    let mut count = 0;
    query!(
        world,
        |_byte: &Byte| {
            count += 1;
        },
        ![Position, Rectangle]
    );
    assert_eq!(count, 1);
}

#[test]
fn correct_single_entity_is_included() {
    let mut world = World::new();

    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();

    let correct_included = EntityID::from_u64(2);
    world
        .create_entity(correct_included, (&BYTE, &POS))
        .unwrap();

    world.create_entity(EntityID::from_u64(3), &BYTE).unwrap();

    query!(world, |entity: EntityID, _rect: &Position, _byte: &Byte| {
        assert_eq!(entity, correct_included);
    });
}

#[test]
fn correct_two_entities_are_included() {
    let mut world = World::new();
    let mut correct_included = HashSet::<_, Global>::default();

    world
        .create_entity(EntityID::from_u64(1), (&BYTE, &RECT))
        .unwrap();

    let id2 = EntityID::from_u64(2);
    world.create_entity(id2, (&BYTE, &POS, &Marked)).unwrap();
    correct_included.insert(id2);

    world.create_entity(EntityID::from_u64(3), &POS).unwrap();

    let id4 = EntityID::from_u64(4);
    world.create_entity(id4, (&BYTE, &Marked)).unwrap();
    correct_included.insert(id4);

    query!(
        world,
        |entity: EntityID, _byte: &mut Byte| {
            assert!(correct_included.remove(&entity));
        },
        [Marked]
    );
    assert!(correct_included.is_empty());
}

#[test]
fn correct_three_entities_are_included() {
    let mut world = World::new();
    let mut correct_included = HashSet::<_, Global>::default();
    let id1 = EntityID::from_u64(1);
    world.create_entity(id1, (&RECT, &POS)).unwrap();
    correct_included.insert(id1);
    world
        .create_entity(EntityID::from_u64(2), (&POS, &RECT, &Marked))
        .unwrap();
    world.create_entity(EntityID::from_u64(3), &RECT).unwrap();
    world.create_entity(EntityID::from_u64(4), &Marked).unwrap();
    let id5 = EntityID::from_u64(5);
    world.create_entity(id5, (&POS, &RECT)).unwrap();
    correct_included.insert(id5);
    world
        .create_entity(EntityID::from_u64(6), (&BYTE, &Marked))
        .unwrap();
    let id7 = EntityID::from_u64(7);
    world.create_entity(id7, (&BYTE, &POS, &RECT)).unwrap();
    correct_included.insert(id7);

    query!(
        world,
        |entity: EntityID, _rect: &mut Rectangle| {
            assert!(correct_included.remove(&entity));
        },
        [Position],
        ![Marked]
    );
    assert!(correct_included.is_empty());
}

#[test]
fn all_entities_are_included_when_no_comps_specified() {
    let mut world = World::new();
    let mut correct_included = HashSet::<_, Global>::default();
    let id1 = EntityID::from_u64(1);
    world.create_entity(id1, (&POS, &RECT, &Marked)).unwrap();
    correct_included.insert(id1);
    let id2 = EntityID::from_u64(2);
    world.create_entity(id2, (&BYTE, &RECT)).unwrap();
    correct_included.insert(id2);
    let id3 = EntityID::from_u64(3);
    world.create_entity(id3, (&BYTE, &POS, &Marked)).unwrap();
    correct_included.insert(id3);
    let id4 = EntityID::from_u64(4);
    world.create_entity(id4, &POS).unwrap();
    correct_included.insert(id4);
    let id5 = EntityID::from_u64(5);
    world.create_entity(id5, (&BYTE, &Marked)).unwrap();
    correct_included.insert(id5);
    let id6 = EntityID::from_u64(6);
    world.create_entity(id6, &BYTE).unwrap();
    correct_included.insert(id6);

    query!(world, |entity: EntityID| {
        assert!(correct_included.remove(&entity));
    });
    assert!(correct_included.is_empty());
}
