#[macro_export]
macro_rules! shader_template_replacements {
    ($($label:literal => $replacement:expr),* $(,)?) => {
        &[$(($label, ($replacement).to_string())),*]
    };
}

#[macro_export]
macro_rules! assert_uniform_valid {
    ($uniform:ident < $( $inner:ty ),+ >) => {
        pastey::item! {
            #[allow(non_upper_case_globals)]
            const  [<_ $uniform _valid>]: () = const {
                assert!(impact_containers::Alignment::SIXTEEN.is_aligned(::std::mem::size_of::<$uniform<$( $inner ),+>>()))
            };
        }
    };
    ($uniform:ty) => {
        pastey::item! {
            #[allow(non_upper_case_globals)]
            const  [<_ $uniform _valid>]: () = const {
                assert!(impact_containers::Alignment::SIXTEEN.is_aligned(::std::mem::size_of::<$uniform>()))
            };
        }
    };
}
