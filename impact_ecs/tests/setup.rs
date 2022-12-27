//!

use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ComponentManager, archetype_of, setup, Component};

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

    let mut manager = ComponentManager::with_initial_components([]).unwrap();

    setup!(manager, || {});

    setup!({}, manager, || {});

    setup!(manager, || {}, [Position]);

    setup!(manager, || {}, [Position], ![LikeByte]);

    setup!(manager, || -> Byte { BYTE });

    setup!(
        {
            let comp = BYTE;
        },
        manager,
        || -> Byte { comp }
    );

    setup!(
        manager,
        || -> (Position, Byte) { (POS, BYTE) },
        [Rectangle],
        ![Marked]
    );

    setup!(manager, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    setup!(
        manager,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    setup!(
        manager,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    setup!(manager, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    setup!(manager, |_byte: &Byte| {});

    setup!(manager, |_pos: &Position, _byte: &Byte| {});

    setup!(manager, |_byte: &Byte| -> Position { POS });

    setup!(manager, |_byte: &Byte| -> (Rectangle, Position) {
        (RECT, POS)
    });

    setup!(manager, |_byte: &Byte| {}, []);

    setup!(manager, |_byte: &Byte| {}, [Position]);

    setup!(manager, |_byte: &Byte| {}, [Position, Rectangle]);

    setup!(manager, |_pos: &Position, _byte: &Byte| {}, [Rectangle]);

    setup!(manager, |_byte: &Byte| {}, ![]);

    setup!(manager, |_byte: &Byte| {}, ![Position]);

    setup!(manager, |_pos: &Position| {}, ![LikeByte]);

    setup!(manager, |_byte: &Byte| {}, ![Position, Rectangle]);

    setup!(manager, |_pos: &Position, _byte: &Byte| {}, ![Rectangle]);

    setup!(manager, |_byte: &Byte| {}, [Position], ![Rectangle]);

    setup!(manager, |_byte: &Byte| {}, ![Position], [Rectangle]);

    setup!(manager, |_byte: &Byte| {}, [Position, Rectangle], ![Marked]);

    setup!(manager, |_byte: &Byte| {}, ![Position, Rectangle], [Marked]);

    // The macro accepts this because it does not know they are
    // the same type, but the result is just that there are no
    // matches
    setup!(manager, |_byte: &LikeByte| {}, ![Byte]);
    setup!(manager, || {}, ![Byte, LikeByte]);
    setup!(manager, || {}, [Byte], ![LikeByte]);

    // This compiles but panics at runtime
    setup!(manager, |_byte: &Byte, _likebyte: &LikeByte| {}, []);
    setup!(manager, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_1() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    setup!(manager, |_byte: &Byte, _likebyte: &LikeByte| {});
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_2() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    setup!(manager, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
fn setup_on_empty_manager_with_no_comp_requirement_runs_nothing() {
    let manager = ComponentManager::with_initial_components([]).unwrap();
    let mut count = 0;
    setup!(manager, || {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_empty_manager_with_comp_requirement_runs_nothing_1() {
    let manager = ComponentManager::with_initial_components([]).unwrap();
    let mut count = 0;
    setup!(manager, |_byte: &Byte| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_empty_manager_with_comp_requirement_runs_nothing_2() {
    let manager = ComponentManager::with_initial_components([]).unwrap();
    let mut count = 0;
    setup!(
        manager,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_manager_with_no_matching_comps_runs_nothing_1() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    setup!(manager, |_pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_manager_with_no_matching_comps_runs_nothing_2() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    setup!(
        manager,
        || {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_manager_missing_one_required_comp_runs_nothing_1() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    setup!(manager, |_byte: &Byte, _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_manager_missing_one_required_comp_runs_nothing_2() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    setup!(
        manager,
        |_byte: &Byte| {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_manager_missing_one_required_comp_runs_nothing_3() {
    let manager = ComponentManager::with_initial_components((&BYTE, &POS)).unwrap();
    let mut count = 0;
    setup!(manager, |_byte: &Byte,
                     _rect: &Rectangle,
                     _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn setup_on_manager_missing_one_required_comp_runs_nothing_4() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    setup!(
        manager,
        |_byte: &Byte| {
            count += 1;
        },
        [Position, Rectangle]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_state_is_available_in_closure() {
    let manager = ComponentManager::with_initial_components(&[BYTE, BYTE]).unwrap();
    let mut count = 0;
    setup!(
        {
            let var_1 = 1;
            let var_2 = 2;
        },
        manager,
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
    let manager = ComponentManager::with_initial_components(&[BYTE, BYTE]).unwrap();
    let mut count = 0;
    let var = 0;
    setup!(
        {
            let var = 1;
        },
        manager,
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
    let manager = ComponentManager::with_initial_components(&[BYTE, BYTE]).unwrap();
    let mut count = 0;
    let mut var = 0;
    setup!(
        {
            var = 1;
        },
        manager,
        || {
            count += 1;
        },
        [Marked]
    );
    assert_eq!(var, 0);
    assert_eq!(count, 0);
}

#[test]
fn setup_on_manager_with_one_disallowed_comp_runs_nothing() {
    let manager = ComponentManager::with_initial_components((&BYTE, &Marked)).unwrap();
    let mut count = 0;
    setup!(
        manager,
        |_byte: &Byte| {
            count += 1;
        },
        ![Marked]
    );
    assert_eq!(count, 0);
}

#[test]
fn setup_on_nonempty_manager_with_no_comp_requirement_works_1() {
    let manager = ComponentManager::with_initial_components(&Marked).unwrap();
    let mut count = 0;
    setup!(manager, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_nonempty_manager_with_no_comp_requirement_works_2() {
    let manager = ComponentManager::with_initial_components((&BYTE, &POS)).unwrap();
    let mut count = 0;
    setup!(manager, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_nonempty_manager_with_no_comp_requirement_works_3() {
    let manager = ComponentManager::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    setup!(manager, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_on_nonempty_manager_with_no_comp_requirement_works_4() {
    let manager =
        ComponentManager::with_initial_components((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let mut count = 0;
    setup!(manager, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn setup_on_manager_with_one_instance_of_one_required_comp_works_1() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    setup!(manager, |byte: &Byte| {
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_manager_with_one_instance_of_one_required_comp_works_2() {
    let manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    setup!(
        manager,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 1);
}

#[test]
fn setup_on_manager_with_two_instances_of_one_required_comp_works() {
    let manager = ComponentManager::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    setup!(manager, |byte: &Byte| {
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
fn setup_on_manager_with_one_instance_of_two_required_comps_works() {
    let manager = ComponentManager::with_initial_components((&BYTE, &POS)).unwrap();
    let mut count = 0;
    setup!(manager, |byte: &Byte, pos: &Position| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_manager_with_one_instance_of_three_required_comps_works() {
    let manager = ComponentManager::with_initial_components((&BYTE, &POS, &RECT)).unwrap();
    let mut count = 0;
    setup!(manager, |byte: &Byte, pos: &Position, rect: &Rectangle| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        assert_eq!(rect, &RECT);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn setup_on_manager_with_two_instances_of_two_required_comps_works() {
    let manager =
        ComponentManager::with_initial_components((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let mut count = 0;
    setup!(manager, |byte: &Byte, pos: &Position| {
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
fn setup_adding_one_zero_size_comp_to_one_comp_one_instance_manager_works() {
    let mut manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    setup!(manager, || -> Marked { Marked }, [Byte]);
    let components = manager.all_components().unwrap();
    assert_eq!(components.archetype(), &archetype_of!(Byte, Marked));
    assert_eq!(components.component_count(), 1);
}

#[test]
fn setup_adding_one_comp_to_one_comp_one_instance_manager_works() {
    let mut manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    setup!(manager, || -> Rectangle { RECT }, [Byte]);
    let manager =
        ComponentManager::with_initial_components(manager.all_components().unwrap()).unwrap();
    assert_eq!(manager.initial_archetype(), &archetype_of!(Byte, Rectangle));
    assert_eq!(manager.initial_component_count(), 1);
    assert_eq!(manager.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(manager.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_one_comp_to_two_comp_one_instance_manager_works() {
    let mut manager = ComponentManager::with_initial_components((&BYTE, &POS)).unwrap();
    setup!(manager, || -> Rectangle { RECT }, [Byte]);
    let manager =
        ComponentManager::with_initial_components(manager.all_components().unwrap()).unwrap();
    assert_eq!(
        manager.initial_archetype(),
        &archetype_of!(Byte, Rectangle, Position)
    );
    assert_eq!(manager.initial_component_count(), 1);
    assert_eq!(manager.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(manager.initial_components::<Position>(), &[POS]);
    assert_eq!(manager.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_two_comps_to_one_comp_one_instance_manager_works() {
    let mut manager = ComponentManager::with_initial_components(&BYTE).unwrap();
    setup!(
        manager,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    let manager =
        ComponentManager::with_initial_components(manager.all_components().unwrap()).unwrap();
    assert_eq!(
        manager.initial_archetype(),
        &archetype_of!(Marked, Byte, Rectangle)
    );
    assert_eq!(manager.initial_component_count(), 1);
    assert_eq!(manager.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(manager.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_two_comps_to_two_comp_one_instance_manager_works() {
    let mut manager = ComponentManager::with_initial_components((&BYTE, &POS)).unwrap();
    setup!(
        manager,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    let manager =
        ComponentManager::with_initial_components(manager.all_components().unwrap()).unwrap();
    assert_eq!(
        manager.initial_archetype(),
        &archetype_of!(Marked, Byte, Rectangle, Position)
    );
    assert_eq!(manager.initial_component_count(), 1);
    assert_eq!(manager.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(manager.initial_components::<Position>(), &[POS]);
    assert_eq!(manager.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn setup_adding_one_comp_to_one_comp_two_instance_manager_works() {
    let mut manager = ComponentManager::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    setup!(
        manager,
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
    let manager =
        ComponentManager::with_initial_components(manager.all_components().unwrap()).unwrap();
    assert_eq!(manager.initial_archetype(), &archetype_of!(Byte, Position));
    assert_eq!(manager.initial_component_count(), 2);
    assert_eq!(manager.initial_components::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(manager.initial_components::<Position>(), &[POS, POS2]);
}

#[test]
fn setup_adding_two_comps_to_one_comp_two_instance_manager_works() {
    let mut manager = ComponentManager::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    setup!(
        manager,
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
    let manager =
        ComponentManager::with_initial_components(manager.all_components().unwrap()).unwrap();
    assert_eq!(
        manager.initial_archetype(),
        &archetype_of!(Byte, Position, Rectangle)
    );
    assert_eq!(manager.initial_component_count(), 2);
    assert_eq!(manager.initial_components::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(manager.initial_components::<Position>(), &[POS, POS2]);
    assert_eq!(manager.initial_components::<Rectangle>(), &[RECT, RECT2]);
}
