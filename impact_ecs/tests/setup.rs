//! Tests for the [`setup`] macro.

use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, archetype_of, setup, Component};

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
#[allow(dead_code)]
fn test_valid_setup_inputs() {
    #![allow(clippy::unnecessary_mut_passed)]

    let mut components: ArchetypeComponentStorage = [].try_into().unwrap();

    setup!(components, || {});

    setup!({}, components, || {});

    setup!(components, || {}, [Position]);

    setup!(components, || {}, [Position], ![LikeByte]);

    setup!(components, || -> Byte { BYTE });

    setup!(
        {
            let comp = BYTE;
        },
        components,
        || -> Byte { comp }
    );

    setup!(
        components,
        || -> (Position, Byte) { (POS, BYTE) },
        [Rectangle],
        ![Marked]
    );

    setup!(components, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    setup!(components, || -> Position { POS }, ![Position]);

    setup!(
        components,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    setup!(
        components,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    setup!(components, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    setup!(components, |_byte: &Byte| {});

    setup!(components, |_pos: &Position, _byte: &Byte| {});

    setup!(components, |_byte: &Byte| -> Position { POS });

    setup!(components, |_pos: &Position| -> Position { POS });

    setup!(components, |_byte: &Byte| -> (Rectangle, Position) {
        (RECT, POS)
    });

    setup!(components, |_byte: &Byte| {}, []);

    setup!(components, |_byte: &Byte| {}, [Position]);

    setup!(components, |_byte: &Byte| {}, [Position, Rectangle]);

    setup!(components, |_pos: &Position, _byte: &Byte| {}, [Rectangle]);

    setup!(components, |_byte: &Byte| {}, ![]);

    setup!(components, |_byte: &Byte| {}, ![Position]);

    setup!(components, |_pos: &Position| {}, ![LikeByte]);

    setup!(components, |_byte: &Byte| {}, ![Position, Rectangle]);

    setup!(components, |_pos: &Position, _byte: &Byte| {}, ![Rectangle]);

    setup!(components, |_byte: &Byte| {}, [Position], ![Rectangle]);

    setup!(components, |_byte: &Byte| {}, ![Position], [Rectangle]);

    setup!(
        components,
        |_byte: &Byte| {},
        [Position, Rectangle],
        ![Marked]
    );

    setup!(
        components,
        |_byte: &Byte| {},
        ![Position, Rectangle],
        [Marked]
    );

    // The macro accepts this because it does not know they are
    // the same type, but the result is just that there are no
    // matches
    setup!(components, |_byte: &LikeByte| {}, ![Byte]);
    setup!(components, || {}, ![Byte, LikeByte]);
    setup!(components, || {}, [Byte], ![LikeByte]);

    // This compiles but panics at runtime
    setup!(components, |_byte: &Byte, _likebyte: &LikeByte| {}, []);
    setup!(components, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_1() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, |_byte: &Byte, _likebyte: &LikeByte| {});
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_2() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
fn setup_on_empty_storage_with_no_comp_requirement_runs_nothing() {
    let components = ArchetypeComponentStorage::empty();
    let mut count = 0;
    setup!(components, || {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_empty_storage_with_comp_requirement_runs_nothing_1() {
    let components = ArchetypeComponentStorage::empty();
    let mut count = 0;
    setup!(components, |_byte: &Byte| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_empty_storage_with_comp_requirement_runs_nothing_2() {
    let components = ArchetypeComponentStorage::empty();
    let mut count = 0;
    setup!(
        components,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_with_no_matching_comps_runs_nothing_1() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(components, |_pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_with_no_matching_comps_runs_nothing_2() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(
        components,
        || {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_1() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(components, |_byte: &Byte, _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_2() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(
        components,
        |_byte: &Byte| {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_3() {
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let mut count = 0;
    setup!(components, |_byte: &Byte,
                        _rect: &Rectangle,
                        _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_storage_missing_one_required_comp_runs_nothing_4() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(
        components,
        |_byte: &Byte| {
            count += 1;
        },
        [Position, Rectangle]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_state_is_available_in_closure() {
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE]);
    let mut count = 0;
    setup!(
        {
            let var_1 = 1;
            let var_2 = 2;
        },
        components,
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
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE]);
    let mut count = 0;
    let var = 0;
    setup!(
        {
            let var = 1;
        },
        components,
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
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE]);
    let mut count = 0;
    let mut var = 0;
    setup!(
        {
            var = 1;
        },
        components,
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
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &Marked)).unwrap();
    let mut count = 0;
    setup!(
        components,
        |_byte: &Byte| {
            count += 1;
        },
        ![Marked]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_1() {
    let components = ArchetypeComponentStorage::from_view(&Marked);
    let mut count = 0;
    setup!(components, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_2() {
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let mut count = 0;
    setup!(components, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_3() {
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut count = 0;
    setup!(components, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_on_nonempty_storage_with_no_comp_requirement_works_4() {
    let components =
        ArchetypeComponentStorage::try_from_view((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let mut count = 0;
    setup!(components, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_on_storage_with_one_instance_of_one_required_comp_works_1() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(components, |byte: &Byte| {
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_one_instance_of_one_required_comp_works_2() {
    let components = ArchetypeComponentStorage::from_view(&BYTE);
    let mut count = 0;
    setup!(
        components,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_two_instances_of_one_required_comp_works() {
    let components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut count = 0;
    setup!(components, |byte: &Byte| {
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
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    let mut count = 0;
    setup!(components, |byte: &Byte, pos: &Position| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_one_instance_of_three_required_comps_works() {
    let components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS, &RECT)).unwrap();
    let mut count = 0;
    setup!(components, |byte: &Byte,
                        pos: &Position,
                        rect: &Rectangle| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        assert_eq!(rect, &RECT);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_storage_with_two_instances_of_two_required_comps_works() {
    let components =
        ArchetypeComponentStorage::try_from_view((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let mut count = 0;
    setup!(components, |byte: &Byte, pos: &Position| {
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
    let mut components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, || -> Marked { Marked }, [Byte]);
    assert_eq!(components.archetype(), &archetype_of!(Byte, Marked));
    assert_eq!(components.n_component_types(), 2);
    assert_eq!(components.component_count(), 1);
}

#[test]
fn setup_adding_one_comp_to_one_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, || -> Rectangle { RECT }, [Byte]);
    assert_eq!(components.archetype(), &archetype_of!(Byte, Rectangle));
    assert_eq!(components.n_component_types(), 2);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(components.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_one_comp_to_two_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    setup!(components, || -> Rectangle { RECT }, [Byte]);
    assert_eq!(
        components.archetype(),
        &archetype_of!(Byte, Rectangle, Position)
    );
    assert_eq!(components.n_component_types(), 3);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(components.components_of_type::<Position>(), &[POS]);
    assert_eq!(components.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_two_comps_to_one_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(
        components,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    assert_eq!(
        components.archetype(),
        &archetype_of!(Marked, Byte, Rectangle)
    );
    assert_eq!(components.n_component_types(), 3);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(components.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_two_comps_to_two_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::try_from_view((&BYTE, &POS)).unwrap();
    setup!(
        components,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    assert_eq!(
        components.archetype(),
        &archetype_of!(Marked, Byte, Rectangle, Position)
    );
    assert_eq!(components.n_component_types(), 4);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE]);
    assert_eq!(components.components_of_type::<Position>(), &[POS]);
    assert_eq!(components.components_of_type::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_one_comp_to_one_comp_two_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut count = 0;
    setup!(
        components,
        || -> Position {
            count += 1;
            if count == 1 {
                POS
            } else {
                POS2
            }
        },
        [Byte]
    );
    assert_eq!(components.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(components.n_component_types(), 2);
    assert_eq!(components.component_count(), 2);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(components.components_of_type::<Position>(), &[POS, POS2]);
}

#[test]
fn setup_adding_two_comps_to_one_comp_two_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut count = 0;
    setup!(
        components,
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
        components.archetype(),
        &archetype_of!(Byte, Position, Rectangle)
    );
    assert_eq!(components.n_component_types(), 3);
    assert_eq!(components.component_count(), 2);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(components.components_of_type::<Position>(), &[POS, POS2]);
    assert_eq!(components.components_of_type::<Rectangle>(), &[RECT, RECT2]);
}

#[test]
fn setup_overwriting_one_comp_in_one_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, || -> Byte { BYTE2 });
    assert_eq!(components.archetype(), &archetype_of!(Byte));
    assert_eq!(components.n_component_types(), 1);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE2]);
}

#[test]
fn setup_overwriting_one_comp_in_one_comp_two_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&[BYTE, BYTE2]);
    let mut count = 0;
    setup!(components, || -> Byte {
        count += 1;
        if count == 1 {
            BYTE2
        } else {
            BYTE
        }
    });
    assert_eq!(components.archetype(), &archetype_of!(Byte));
    assert_eq!(components.n_component_types(), 1);
    assert_eq!(components.component_count(), 2);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE2, BYTE]);
}

#[test]
fn setup_overwriting_one_comp_in_two_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::try_from_view((&POS, &BYTE)).unwrap();
    setup!(components, || -> Byte { BYTE2 });
    assert_eq!(components.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(components.n_component_types(), 2);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE2]);
    assert_eq!(components.components_of_type::<Position>(), &[POS]);
}

#[test]
fn setup_overwriting_one_comp_in_two_comp_two_instance_storage_works() {
    let mut components =
        ArchetypeComponentStorage::try_from_view((&[POS, POS2], &[BYTE, BYTE2])).unwrap();
    let mut count = 0;
    setup!(components, || -> Byte {
        count += 1;
        if count == 1 {
            BYTE2
        } else {
            BYTE
        }
    });
    assert_eq!(components.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(components.n_component_types(), 2);
    assert_eq!(components.component_count(), 2);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE2, BYTE]);
    assert_eq!(components.components_of_type::<Position>(), &[POS, POS2]);
}

#[test]
fn setup_overwriting_one_included_comp_in_one_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, || -> Byte { BYTE2 }, [Byte]);
    assert_eq!(components.archetype(), &archetype_of!(Byte));
    assert_eq!(components.n_component_types(), 1);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE2]);
}

#[test]
fn setup_overwriting_one_arg_included_comp_in_one_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, |_byte: &Byte| -> Byte { BYTE2 });
    assert_eq!(components.archetype(), &archetype_of!(Byte));
    assert_eq!(components.n_component_types(), 1);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE2]);
}

#[test]
fn setup_adding_and_overwriting_two_comps_in_one_comp_one_instance_storage_works() {
    let mut components = ArchetypeComponentStorage::from_view(&BYTE);
    setup!(components, || -> (Position, Byte) { (POS, BYTE2) });
    assert_eq!(components.archetype(), &archetype_of!(Byte, Position));
    assert_eq!(components.n_component_types(), 2);
    assert_eq!(components.component_count(), 1);
    assert_eq!(components.components_of_type::<Byte>(), &[BYTE2]);
    assert_eq!(components.components_of_type::<Position>(), &[POS]);
}
