//!

use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeCompExtender, archetype_of, prepare, Component};

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

/// These prepare invocations should all compile successfully.
#[allow(dead_code)]
fn test_valid_prepare_inputs() {
    #![allow(clippy::unnecessary_mut_passed)]

    let mut extender = ArchetypeCompExtender::with_initial_components([]).unwrap();

    prepare!(extender, || {});

    prepare!(extender, || {}, [Position]);

    prepare!(extender, || {}, [Position], ![LikeByte]);

    prepare!(extender, || -> Byte { BYTE });

    prepare!(
        extender,
        || -> (Position, Byte) { (POS, BYTE) },
        [Rectangle],
        ![Marked]
    );

    prepare!(extender, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    prepare!(
        extender,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    prepare!(
        extender,
        |_pos: &Position| -> Byte { BYTE },
        [Rectangle],
        ![Marked]
    );

    prepare!(extender, |_pos: &Position| -> Byte { BYTE }, [Rectangle]);

    prepare!(extender, |_byte: &Byte| {});

    prepare!(extender, |_pos: &Position, _byte: &Byte| {});

    prepare!(extender, |_byte: &Byte| -> Position { POS });

    prepare!(extender, |_byte: &Byte| -> (Rectangle, Position) {
        (RECT, POS)
    });

    prepare!(extender, |_byte: &Byte| {}, []);

    prepare!(extender, |_byte: &Byte| {}, [Position]);

    prepare!(extender, |_byte: &Byte| {}, [Position, Rectangle]);

    prepare!(extender, |_pos: &Position, _byte: &Byte| {}, [Rectangle]);

    prepare!(extender, |_byte: &Byte| {}, ![]);

    prepare!(extender, |_byte: &Byte| {}, ![Position]);

    prepare!(extender, |_pos: &Position| {}, ![LikeByte]);

    prepare!(extender, |_byte: &Byte| {}, ![Position, Rectangle]);

    prepare!(extender, |_pos: &Position, _byte: &Byte| {}, ![Rectangle]);

    prepare!(extender, |_byte: &Byte| {}, [Position], ![Rectangle]);

    prepare!(extender, |_byte: &Byte| {}, ![Position], [Rectangle]);

    prepare!(
        extender,
        |_byte: &Byte| {},
        [Position, Rectangle],
        ![Marked]
    );

    prepare!(
        extender,
        |_byte: &Byte| {},
        ![Position, Rectangle],
        [Marked]
    );

    // The macro accepts this because it does not know they are
    // the same type, but the result is just that there are no
    // matches
    prepare!(extender, |_byte: &LikeByte| {}, ![Byte]);
    prepare!(extender, || {}, ![Byte, LikeByte]);
    prepare!(extender, || {}, [Byte], ![LikeByte]);

    // This compiles but panics at runtime
    prepare!(extender, |_byte: &Byte, _likebyte: &LikeByte| {}, []);
    prepare!(extender, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_1() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    prepare!(extender, |_byte: &Byte, _likebyte: &LikeByte| {});
}

#[test]
#[should_panic]
fn requiring_aliased_comps_fails_2() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    prepare!(extender, |_byte: &Byte| {}, [LikeByte]);
}

#[test]
fn prepare_on_empty_extender_with_no_comp_requirement_runs_nothing() {
    let extender = ArchetypeCompExtender::with_initial_components([]).unwrap();
    let mut count = 0;
    prepare!(extender, || {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_empty_extender_with_comp_requirement_runs_nothing_1() {
    let extender = ArchetypeCompExtender::with_initial_components([]).unwrap();
    let mut count = 0;
    prepare!(extender, |_byte: &Byte| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_empty_extender_with_comp_requirement_runs_nothing_2() {
    let extender = ArchetypeCompExtender::with_initial_components([]).unwrap();
    let mut count = 0;
    prepare!(
        extender,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_extender_with_no_matching_comps_runs_nothing_1() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    prepare!(extender, |_pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_extender_with_no_matching_comps_runs_nothing_2() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    prepare!(
        extender,
        || {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_extender_missing_one_required_comp_runs_nothing_1() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    prepare!(extender, |_byte: &Byte, _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_extender_missing_one_required_comp_runs_nothing_2() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    prepare!(
        extender,
        |_byte: &Byte| {
            count += 1;
        },
        [Position]
    );
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_extender_missing_one_required_comp_runs_nothing_3() {
    let extender = ArchetypeCompExtender::with_initial_components((&BYTE, &POS)).unwrap();
    let mut count = 0;
    prepare!(extender, |_byte: &Byte,
                        _rect: &Rectangle,
                        _pos: &Position| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_extender_missing_one_required_comp_runs_nothing_4() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    prepare!(
        extender,
        |_byte: &Byte| {
            count += 1;
        },
        [Position, Rectangle]
    );
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_extender_with_one_disallowed_comp_runs_nothing() {
    let extender = ArchetypeCompExtender::with_initial_components((&BYTE, &Marked)).unwrap();
    let mut count = 0;
    prepare!(
        extender,
        |_byte: &Byte| {
            count += 1;
        },
        ![Marked]
    );
    assert_eq!(count, 0);
}

#[test]
fn prepare_on_nonempty_extender_with_no_comp_requirement_works_1() {
    let extender = ArchetypeCompExtender::with_initial_components(&Marked).unwrap();
    let mut count = 0;
    prepare!(extender, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn prepare_on_nonempty_extender_with_no_comp_requirement_works_2() {
    let extender = ArchetypeCompExtender::with_initial_components((&BYTE, &POS)).unwrap();
    let mut count = 0;
    prepare!(extender, || {
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn prepare_on_nonempty_extender_with_no_comp_requirement_works_3() {
    let extender = ArchetypeCompExtender::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    prepare!(extender, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn prepare_on_nonempty_extender_with_no_comp_requirement_works_4() {
    let extender =
        ArchetypeCompExtender::with_initial_components((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let mut count = 0;
    prepare!(extender, || {
        count += 1;
    });
    assert_eq!(count, 2);
}

#[test]
fn prepare_on_extender_with_one_instance_of_one_required_comp_works_1() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    prepare!(extender, |byte: &Byte| {
        assert_eq!(byte, &BYTE);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn prepare_on_extender_with_one_instance_of_one_required_comp_works_2() {
    let extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    let mut count = 0;
    prepare!(
        extender,
        || {
            count += 1;
        },
        [Byte]
    );
    assert_eq!(count, 1);
}

#[test]
fn prepare_on_extender_with_two_instances_of_one_required_comp_works() {
    let extender = ArchetypeCompExtender::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    prepare!(extender, |byte: &Byte| {
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
fn prepare_on_extender_with_one_instance_of_two_required_comps_works() {
    let extender = ArchetypeCompExtender::with_initial_components((&BYTE, &POS)).unwrap();
    let mut count = 0;
    prepare!(extender, |byte: &Byte, pos: &Position| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn prepare_on_extender_with_one_instance_of_three_required_comps_works() {
    let extender = ArchetypeCompExtender::with_initial_components((&BYTE, &POS, &RECT)).unwrap();
    let mut count = 0;
    prepare!(extender, |byte: &Byte, pos: &Position, rect: &Rectangle| {
        assert_eq!(byte, &BYTE);
        assert_eq!(pos, &POS);
        assert_eq!(rect, &RECT);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn prepare_on_extender_with_two_instances_of_two_required_comps_works() {
    let extender =
        ArchetypeCompExtender::with_initial_components((&[BYTE, BYTE2], &[POS, POS2])).unwrap();
    let mut count = 0;
    prepare!(extender, |byte: &Byte, pos: &Position| {
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
fn prepare_adding_one_zero_size_comp_to_one_comp_one_instance_extender_works() {
    let mut extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    prepare!(extender, || -> Marked { Marked }, [Byte]);
    let components = extender.all_components().unwrap();
    assert_eq!(
        components.archetype(),
        &archetype_of!(Byte, Marked).unwrap()
    );
    assert_eq!(components.component_count(), 1);
}

#[test]
fn prepare_adding_one_comp_to_one_comp_one_instance_extender_works() {
    let mut extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    prepare!(extender, || -> Rectangle { RECT }, [Byte]);
    let extender =
        ArchetypeCompExtender::with_initial_components(extender.all_components().unwrap()).unwrap();
    assert_eq!(
        extender.initial_archetype(),
        &archetype_of!(Byte, Rectangle).unwrap()
    );
    assert_eq!(extender.initial_component_count(), 1);
    assert_eq!(extender.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(extender.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn prepare_adding_one_comp_to_two_comp_one_instance_extender_works() {
    let mut extender = ArchetypeCompExtender::with_initial_components((&BYTE, &POS)).unwrap();
    prepare!(extender, || -> Rectangle { RECT }, [Byte]);
    let extender =
        ArchetypeCompExtender::with_initial_components(extender.all_components().unwrap()).unwrap();
    assert_eq!(
        extender.initial_archetype(),
        &archetype_of!(Byte, Rectangle, Position).unwrap()
    );
    assert_eq!(extender.initial_component_count(), 1);
    assert_eq!(extender.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(extender.initial_components::<Position>(), &[POS]);
    assert_eq!(extender.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn prepare_adding_two_comps_to_one_comp_one_instance_extender_works() {
    let mut extender = ArchetypeCompExtender::with_initial_components(&BYTE).unwrap();
    prepare!(
        extender,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    let extender =
        ArchetypeCompExtender::with_initial_components(extender.all_components().unwrap()).unwrap();
    assert_eq!(
        extender.initial_archetype(),
        &archetype_of!(Marked, Byte, Rectangle).unwrap()
    );
    assert_eq!(extender.initial_component_count(), 1);
    assert_eq!(extender.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(extender.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn prepare_adding_two_comps_to_two_comp_one_instance_extender_works() {
    let mut extender = ArchetypeCompExtender::with_initial_components((&BYTE, &POS)).unwrap();
    prepare!(
        extender,
        || -> (Rectangle, Marked) { (RECT, Marked) },
        [Byte]
    );
    let extender =
        ArchetypeCompExtender::with_initial_components(extender.all_components().unwrap()).unwrap();
    assert_eq!(
        extender.initial_archetype(),
        &archetype_of!(Marked, Byte, Rectangle, Position).unwrap()
    );
    assert_eq!(extender.initial_component_count(), 1);
    assert_eq!(extender.initial_components::<Byte>(), &[BYTE]);
    assert_eq!(extender.initial_components::<Position>(), &[POS]);
    assert_eq!(extender.initial_components::<Rectangle>(), &[RECT]);
}

#[test]
fn prepare_adding_one_comp_to_one_comp_two_instance_extender_works() {
    let mut extender = ArchetypeCompExtender::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    prepare!(
        extender,
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
    let extender =
        ArchetypeCompExtender::with_initial_components(extender.all_components().unwrap()).unwrap();
    assert_eq!(
        extender.initial_archetype(),
        &archetype_of!(Byte, Position).unwrap()
    );
    assert_eq!(extender.initial_component_count(), 2);
    assert_eq!(extender.initial_components::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(extender.initial_components::<Position>(), &[POS, POS2]);
}

#[test]
fn prepare_adding_two_comps_to_one_comp_two_instance_extender_works() {
    let mut extender = ArchetypeCompExtender::with_initial_components(&[BYTE, BYTE2]).unwrap();
    let mut count = 0;
    prepare!(
        extender,
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
    let extender =
        ArchetypeCompExtender::with_initial_components(extender.all_components().unwrap()).unwrap();
    assert_eq!(
        extender.initial_archetype(),
        &archetype_of!(Byte, Position, Rectangle).unwrap()
    );
    assert_eq!(extender.initial_component_count(), 2);
    assert_eq!(extender.initial_components::<Byte>(), &[BYTE, BYTE2]);
    assert_eq!(extender.initial_components::<Position>(), &[POS, POS2]);
    assert_eq!(extender.initial_components::<Rectangle>(), &[RECT, RECT2]);
}
