//! Tests for the [`setup`] macro.

use bytemuck::{Pod, Zeroable};
use impact_ecs::{
    Component, archetype::ArchetypeComponentStorage, archetype_of, setup, world::PrototypeEntities,
};
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
const RECT2: Rectangle = Rectangle {
    center: [5.2, 0.2],
    dimensions: [3.1, 9.8],
};

/// These setup invocations should all compile successfully.
#[allow(dead_code, clippy::unnecessary_mut_passed)]
fn test_valid_setup_inputs() {
    let ids = Vec::new();
    let components = ArchetypeComponentStorage::empty();
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || {});

    setup!({}, entities, || {});

    setup!(entities, || {}, [Position]);

    setup!(entities, || {}, [Position], ![LikeByte]);

    setup!(entities, || -> () {});

    setup!(entities, || -> Byte { BYTE });

    setup!(entities, || -> Marked { Marked });

    setup!(
        {
            let comp = BYTE;
        },
        entities,
        || -> Byte { comp }
    );

    setup!(
        entities,
        || -> (Position, Byte) { (POS, BYTE) },
        [Rectangle],
        ![Marked]
    );

    setup!(entities, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    setup!(entities, || -> Position { POS }, ![Position]);

    setup!(
        entities,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    setup!(
        entities,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    setup!(entities, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    setup!(entities, |_byte: &Byte| {});

    setup!(entities, |_pos: &Position, _byte: &Byte| {});

    setup!(entities, |_byte: &Byte| -> Position { POS });

    setup!(entities, |_pos: &Position| -> Position { POS });

    setup!(entities, |_byte: &Byte| -> (Rectangle, Position) {
        (RECT, POS)
    });

    setup!(entities, |_byte: &Byte| {}, []);

    setup!(entities, |_byte: &Byte| {}, [Position]);

    setup!(entities, |_byte: &Byte| {}, [Position, Rectangle]);

    setup!(entities, |_pos: &Position, _byte: &Byte| {}, [Rectangle]);

    setup!(entities, |_byte: &Byte| {}, ![]);

    setup!(entities, |_byte: &Byte| {}, ![Position]);

    setup!(entities, |_pos: &Position| {}, ![LikeByte]);

    setup!(entities, |_byte: &Byte| {}, ![Position, Rectangle]);

    setup!(entities, |_pos: &Position, _byte: &Byte| {}, ![Rectangle]);

    setup!(entities, |_byte: &Byte| {}, [Position], ![Rectangle]);

    setup!(entities, |_byte: &Byte| {}, ![Position], [Rectangle]);

    setup!(
        entities,
        |_byte: &Byte| {},
        [Position, Rectangle],
        ![Marked]
    );

    setup!(
        entities,
        |_byte: &Byte| {},
        ![Position, Rectangle],
        [Marked]
    );

    setup!(entities, |_byte: Option<&Byte>| {});

    setup!(entities, |_byte: Option<&Byte>, _pos: Option<&Position>| {});

    setup!(entities, |_byte: &Byte, _pos: Option<&Position>| {});

    setup!(entities, |_byte: Option<&Byte>, _pos: &Position| {});

    setup!(entities, |_byte: Option<&Byte>,
                      _pos: &Position|
     -> Marked { Marked });

    setup!(
        entities,
        |_byte: Option<&Byte>, _pos: &Position| -> Marked { Marked },
        [Rectangle]
    );

    setup!(
        entities,
        |_byte: Option<&Byte>, _pos: &Position| -> Marked { Marked },
        [Rectangle],
        ![Marked]
    );

    let _: Result<(), ()> = setup!(entities, || -> Result<(), ()> { Ok(()) });

    let _: Result<(), ()> = setup!(entities, || -> Result<Byte, ()> { Ok(BYTE) });

    let _: Result<(), i32> = setup!(entities, || -> Result<Byte, i32> { Err(1) });

    let _: Result<(), ()> = setup!(entities, || -> Result<(Byte, Position), ()> { Err(()) });

    let _: anyhow::Result<()> = setup!(entities, || -> anyhow::Result<Byte> { Ok(BYTE) });

    let _: anyhow::Result<()> = setup!(entities, || -> ::anyhow::Result<Byte> { Ok(BYTE) });

    setup!(entities, |_entity_id: EntityID| {});
    setup!(entities, |_entity_id: EntityID, _byte: &Byte| {});

    // The macro accepts this because it does not know they are
    // the same type, but the result is just that there are no
    // matches
    setup!(entities, |_byte: &LikeByte| {}, ![Byte]);
    setup!(entities, || {}, ![Byte, LikeByte]);
    setup!(entities, || {}, [Byte], ![LikeByte]);

    // This compiles but panics at runtime
    setup!(entities, |_byte: &Byte, _likebyte: &LikeByte| {}, []);
    setup!(entities, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_1() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, |_byte: &Byte, _likebyte: &LikeByte| {});
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_2() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
fn setup_on_empty_storage_with_no_comp_requirement_runs_nothing() {
    let ids = Vec::new();
    let components = ArchetypeComponentStorage::empty();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, || {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_empty_storage_with_comp_requirement_runs_nothing_1() {
    let ids = Vec::new();
    let components = ArchetypeComponentStorage::empty();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |_byte: &Byte| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_empty_storage_with_comp_requirement_runs_nothing_2() {
    let ids = Vec::new();
    let components = ArchetypeComponentStorage::empty();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_with_no_matching_comps_runs_nothing_1() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |_pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_with_no_matching_comps_runs_nothing_2() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        || {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_1() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |_byte: &Byte, _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_2() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        |_byte: &Byte| {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_3() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |_byte: &Byte,
                      _rect: &Rectangle,
                      _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_4() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        |_byte: &Byte| {
            count += 1;
        },
        [Position, Rectangle]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_state_is_available_in_closure() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE]);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        {
            let var_1 = 1;
            let var_2 = 2;
        },
        entities,
        || {
            assert_eq!(var_1, 1);
            assert_eq!(var_2, 2);
            count += 1;
        }
    );
    assert_eq!(count, 2);
}

#[test]
fn setup_state_is_unavailable_after_closure() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE]);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    let var = 0;
    setup!(
        {
            let var = 1;
        },
        entities,
        || {
            assert_eq!(var, 1);
            count += 1;
        }
    );
    assert_eq!(var, 0);
    assert_eq!(count, 2);
}

#[test]
fn setup_state_is_not_run_if_closure_is_not_run() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE]);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    let mut var = 0;
    setup!(
        {
            var = 1;
        },
        entities,
        || {
            count += 1;
        },
        [Marked]
    );
    assert_eq!(var, 0);
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_with_one_disallowed_comp_runs_nothing() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &Marked)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        |_byte: &Byte| {
            count += 1;
        },
        ![Marked]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_1() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&Marked);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_2() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_3() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_4() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components =
        ArchetypeComponentStorage::try_from_view((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_on_storage_with_one_instance_of_one_required_comp_works_1() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: &Byte| {
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_one_instance_of_one_required_comp_works_2() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_two_instances_of_one_required_comp_works() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: &Byte| {
        if count == 0 {
            assert_eq!(byte, &BYTE);
        } else {
            assert_eq!(byte, &BYTE2);
        }
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_on_storage_with_one_instance_of_two_required_comps_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: &Byte, pos: &Position| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_one_instance_of_three_required_comps_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS, &RECT)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: &Byte, pos: &Position, rect: &Rectangle| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        assert_eq!(rect, &RECT);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_two_instances_of_two_required_comps_works() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components =
        ArchetypeComponentStorage::try_from_view((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: &Byte, pos: &Position| {
        if count == 0 {
            assert_eq!(byte, &BYTE);
            assert_eq!(pos, &POS);
        } else {
            assert_eq!(byte, &BYTE2);
            assert_eq!(pos, &POS2);
        }
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_adding_one_zero_size_comp_to_one_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || -> Marked { Marked }, [Byte]);
    assert_eq!(entities.archetype(), &archetype_of!(Byte, Marked));
    assert_eq!(entities.n_component_types(), 2);
    assert_eq!(entities.count(), 1);
}

#[test]
fn setup_adding_one_comp_to_one_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || -> Rectangle { RECT }, [Byte]);
    assert_eq!(entities.archetype(), &archetype_of!(Byte, Rectangle));
    assert_eq!(entities.n_component_types(), 2);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(entities.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_one_comp_to_two_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || -> Rectangle { RECT }, [Byte]);
    assert_eq!(
        entities.archetype(),
        &archetype_of!(Byte, Rectangle, Position)
    );
    assert_eq!(entities.n_component_types(), 3);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(entities.components_of_type::<Position>(), &[POS]);
    assert_eq!(entities.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_two_comps_to_one_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(
        entities,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    assert_eq!(
        entities.archetype(),
        &archetype_of!(Marked, Byte, Rectangle)
    );
    assert_eq!(entities.n_component_types(), 3);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(entities.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_two_comps_to_two_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(
        entities,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    assert_eq!(
        entities.archetype(),
        &archetype_of!(Marked, Byte, Rectangle, Position)
    );
    assert_eq!(entities.n_component_types(), 4);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(entities.components_of_type::<Position>(), &[POS]);
    assert_eq!(entities.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_one_comp_to_one_comp_two_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        || -> Position {
            count += 1;
            if count == 1 { POS } else { POS2 }
        },
        [Byte]
    );
    assert_eq!(entities.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(entities.n_component_types(), 2);
    assert_eq!(entities.count(), 2);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(entities.components_of_type::<Position>(), &[POS, POS2]);
}

#[test]
fn setup_adding_two_comps_to_one_comp_two_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        || -> (Position, Rectangle) {
            count += 1;
            if count == 1 {
                (POS, RECT)
            } else {
                (POS2, RECT2)
            }
        },
        [Byte]
    );
    assert_eq!(
        entities.archetype(),
        &archetype_of!(Byte, Position, Rectangle)
    );
    assert_eq!(entities.n_component_types(), 3);
    assert_eq!(entities.count(), 2);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(entities.components_of_type::<Position>(), &[POS, POS2]);
    assert_eq!(entities.components_of_type::<Rectangle>(), &[RECT, RECT2]);
}

#[test]
fn setup_overwriting_one_comp_in_one_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || -> Byte { BYTE2 });
    assert_eq!(entities.archetype(), &archetype_of!(Byte));
    assert_eq!(entities.n_component_types(), 1);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE2]);
}

#[test]
fn setup_overwriting_one_comp_in_one_comp_two_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, || -> Byte {
        count += 1;
        if count == 1 { BYTE2 } else { BYTE }
    });
    assert_eq!(entities.archetype(), &archetype_of!(Byte));
    assert_eq!(entities.n_component_types(), 1);
    assert_eq!(entities.count(), 2);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE2, BYTE]);
}

#[test]
fn setup_overwriting_one_comp_in_two_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&POS, &BYTE)).unwrap();
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || -> Byte { BYTE2 });
    assert_eq!(entities.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(entities.n_component_types(), 2);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE2]);
    assert_eq!(entities.components_of_type::<Position>(), &[POS]);
}

#[test]
fn setup_overwriting_one_comp_in_two_comp_two_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0), EntityID::from_u64(1)];
    let components =
        ArchetypeComponentStorage::try_from_view((&[POS, POS2], &[BYTE, BYTE2])).unwrap();
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, || -> Byte {
        count += 1;
        if count == 1 { BYTE2 } else { BYTE }
    });
    assert_eq!(entities.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(entities.n_component_types(), 2);
    assert_eq!(entities.count(), 2);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE2, BYTE]);
    assert_eq!(entities.components_of_type::<Position>(), &[POS, POS2]);
}

#[test]
fn setup_overwriting_one_included_comp_in_one_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || -> Byte { BYTE2 }, [Byte]);
    assert_eq!(entities.archetype(), &archetype_of!(Byte));
    assert_eq!(entities.n_component_types(), 1);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE2]);
}

#[test]
fn setup_overwriting_one_arg_included_comp_in_one_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, |_byte: &Byte| -> Byte { BYTE2 });
    assert_eq!(entities.archetype(), &archetype_of!(Byte));
    assert_eq!(entities.n_component_types(), 1);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE2]);
}

#[test]
fn setup_adding_and_overwriting_two_comps_in_one_comp_one_instance_storage_works() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    setup!(entities, || -> (Position, Byte) { (POS, BYTE2) });
    assert_eq!(entities.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(entities.n_component_types(), 2);
    assert_eq!(entities.count(), 1);
    assert_eq!(entities.components_of_type::<Byte>(), &[BYTE2]);
    assert_eq!(entities.components_of_type::<Position>(), &[POS]);
}

#[test]
fn setup_requesting_optional_comp_from_empty_storage_runs_nothing() {
    let ids = Vec::new();
    let components = ArchetypeComponentStorage::empty();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |_pos: Option<&Position>| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_requesting_optional_and_required_comp_from_empty_storage_runs_nothing_1() {
    let ids = Vec::new();
    let components = ArchetypeComponentStorage::empty();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |_pos: Option<&Position>, _byte: &Byte| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_requesting_optional_and_required_comp_from_empty_storage_runs_nothing_2() {
    let ids = Vec::new();
    let components = ArchetypeComponentStorage::empty();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        |_pos: Option<&Position>| {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_optional_comp_gives_none() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |pos: Option<&Position>| {
        assert!(pos.is_none());
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_missing_two_optional_comps_gives_none() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |pos: Option<&Position>,
                      rect: Option<&Rectangle>| {
        assert!(pos.is_none());
        assert!(rect.is_none());
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_missing_optional_and_matching_required_comp_gives_none_1() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |_byte: &Byte, pos: Option<&Position>| {
        assert!(pos.is_none());
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_missing_optional_and_matching_required_comp_gives_none_2() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(
        components,
        |pos: Option<&Position>| {
            assert!(pos.is_none());
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_optional_comp_gives_some() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: Option<&Byte>| {
        assert_eq!(byte.unwrap(), &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_optional_and_missing_comp_gives_some_and_none() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: Option<&Byte>, pos: Option<&Position>| {
        assert_eq!(byte.unwrap(), &BYTE);
        assert!(pos.is_none());
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_two_optional_comps_gives_some() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&POS, &BYTE)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: Option<&Byte>, pos: Option<&Position>| {
        assert_eq!(byte.unwrap(), &BYTE);
        assert_eq!(pos.unwrap(), &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_optional_and_required_comps_gives_some_1() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&POS, &BYTE)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |byte: &Byte, pos: Option<&Position>| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos.unwrap(), &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_optional_and_required_comps_gives_some_2() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::try_from_view((&POS, &BYTE)).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(
        entities,
        |pos: Option<&Position>| {
            assert_eq!(pos.unwrap(), &POS);
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_no_matching_comps_does_not_return_closure_error() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let res = setup!(entities, |_pos: &Position| -> Result<(), i32> { Err(1) });
    assert_eq!(res, Ok(()));
}

#[test]
fn setup_on_storage_with_matching_comp_returns_closure_error() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let res = setup!(entities, |_byte: &Byte| -> Result<(), i32> { Err(1) });
    assert_eq!(res, Err(1));
}

#[test]
fn setup_on_storage_with_matching_comp_has_no_effect_if_closure_errors() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    let res = setup!(entities, |_byte: &Byte| -> Result<Position, i32> { Err(1) });
    assert_eq!(res, Err(1));
    assert_eq!(entities.n_component_types(), 1);
    assert!(entities.has_component_type::<Byte>());
}

#[test]
fn setup_on_storage_with_matching_comp_returns_ok_if_closure_does() {
    let ids = vec![EntityID::from_u64(0)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut entities = PrototypeEntities::new(ids, components).unwrap();

    let res = setup!(entities, |_byte: &Byte| -> Result<Position, i32> {
        Ok(POS)
    });
    assert_eq!(res, Ok(()));
}

#[test]
fn setup_entity_id_arg_provides_correct_id_for_one_instance() {
    let ids = vec![EntityID::from_u64(42)];
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |entity_id: EntityID, byte: &Byte| {
        assert_eq!(entity_id, EntityID::from_u64(42));
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_entity_id_arg_provides_correct_ids_for_two_instances() {
    let ids = vec![EntityID::from_u64(10), EntityID::from_u64(20)];
    let components =
        ArchetypeComponentStorage::try_from_view((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let entities = PrototypeEntities::new(ids, components).unwrap();

    let mut count = 0;
    setup!(entities, |entity_id: EntityID,
                      byte: &Byte,
                      pos: &Position| {
        if count == 0 {
            assert_eq!(entity_id, EntityID::from_u64(10));
            assert_eq!(byte, &BYTE);
            assert_eq!(pos, &POS);
        } else {
            assert_eq!(entity_id, EntityID::from_u64(20));
            assert_eq!(byte, &BYTE2);
            assert_eq!(pos, &POS2);
        }
        count += 1;
    });
    assert_eq!(count, 2);
}
