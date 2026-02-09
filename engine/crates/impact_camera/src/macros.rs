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

/// Defines a setup value that derives`impact_ecs::SetupComponent` if the `ecs`
/// feature is enabled.
#[macro_export]
macro_rules! define_setup_type {
    (
        target = $target:ident ;
        $(#[$outer:meta])*
        $vis:vis struct $name:ident $($rest:tt)*
    ) => {
        $(#[$outer])*
        #[cfg_attr(
            feature = "ecs",
            doc = concat!(
                "\n\n\
                This is a [`SetupComponent`](impact_ecs::component::SetupComponent) \
                whose purpose is to aid in constructing a `", stringify!($target),
                "` component for an entity. It is therefore not kept after entity \
                creation."
            )
        )]
        #[cfg_attr(feature = "ecs", derive(impact_ecs::SetupComponent))]
        $vis struct $name $($rest)*
    };

    (
        $(#[$outer:meta])*
        $vis:vis struct $name:ident $($rest:tt)*
    ) => {
        $(#[$outer])*
        #[cfg_attr(feature = "ecs", derive(impact_ecs::SetupComponent))]
        $vis struct $name $($rest)*
    };
}
