//! Crate-local macros and utility macros.

/// Defines a type that derives`impact_ecs::Component` if the `ecs` feature is
/// enabled.
#[macro_export]
macro_rules! define_component_type {
    (
        $(#[$outer:meta])*
        $vis:vis struct $name:ident $($rest:tt)*
    ) => {
        $(#[$outer])*
        #[cfg_attr(
            feature = "ecs",
            doc = concat!(
                "\n\n\
                This is an ECS [`Component`](impact_ecs::component::Component)."
            )
        )]
        #[cfg_attr(feature = "ecs", derive(impact_ecs::Component))]
        $vis struct $name $($rest)*
    };
}
