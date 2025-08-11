//! ECS macros.

/// Registers a
/// [`ComponentFlagDeclaration`](crate::component::ComponentFlagDeclaration)
/// with the given flags for the given component type.
#[macro_export]
macro_rules! declare_component_flags {
    ($component:ty, $flags:expr) => {
        inventory::submit! {
            $crate::component::ComponentFlagDeclaration {
                id: <$component as $crate::component::Component>::COMPONENT_ID,
                flags: $flags,
            }
        }
    };
}
