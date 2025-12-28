//! Crate-local macros and utility macros.

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

macro_rules! impl_binop {
    ($op:ident, $method:ident, $tl:ty, $tr:ty, $to:ty, |$lhs:ident, $rhs:ident| $body:block) => {
        impl<'a> ::std::ops::$op<&'a $tr> for &'a $tl {
            type Output = $to;

            #[inline]
            fn $method(self, rhs: &'a $tr) -> Self::Output {
                let $lhs = self;
                let $rhs = rhs;
                $body
            }
        }

        impl ::std::ops::$op<$tr> for &$tl {
            type Output = $to;

            #[inline]
            fn $method(self, rhs: $tr) -> Self::Output {
                self.$method(&rhs)
            }
        }

        impl<'a> ::std::ops::$op<&'a $tr> for $tl {
            type Output = $to;

            #[inline]
            fn $method(self, rhs: &'a $tr) -> Self::Output {
                (&self).$method(rhs)
            }
        }

        impl ::std::ops::$op<$tr> for $tl {
            type Output = $to;

            #[inline]
            fn $method(self, rhs: $tr) -> Self::Output {
                (&self).$method(&rhs)
            }
        }
    };
}

macro_rules! impl_unary_op {
    ($op:ident, $method:ident, $t:ty, $to:ty, |$this:ident| $body:block) => {
        impl ::std::ops::$op for &$t {
            type Output = $to;

            #[inline]
            fn $method(self) -> Self::Output {
                let $this = self;
                $body
            }
        }

        impl ::std::ops::$op for $t {
            type Output = $to;

            #[inline]
            fn $method(self) -> Self::Output {
                (&self).$method()
            }
        }
    };
}

macro_rules! impl_binop_assign {
    ($op:ident, $method:ident, $tl:ty, $tr:ty, |$lhs:ident, $rhs:ident| $body:block) => {
        impl ::std::ops::$op<&$tr> for $tl {
            #[inline]
            fn $method(&mut self, rhs: &$tr) {
                let $lhs = self;
                let $rhs = rhs;
                $body
            }
        }

        impl ::std::ops::$op<$tr> for $tl {
            #[inline]
            fn $method(&mut self, rhs: $tr) {
                self.$method(&rhs);
            }
        }
    };
}

macro_rules! impl_abs_diff_eq {
    ($t:ty, |$arg1:ident, $arg2:ident, $arg3:ident| $body:block) => {
        impl ::approx::AbsDiffEq for $t {
            type Epsilon = f32;

            fn default_epsilon() -> Self::Epsilon {
                f32::default_epsilon()
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
                let $arg1 = self;
                let $arg2 = other;
                let $arg3 = epsilon;
                $body
            }
        }
    };
}

macro_rules! impl_relative_eq {
    ($t:ty, |$arg1:ident, $arg2:ident, $arg3:ident, $arg4:ident| $body:block) => {
        impl ::approx::RelativeEq for $t {
            fn default_max_relative() -> Self::Epsilon {
                f32::default_max_relative()
            }

            fn relative_eq(
                &self,
                other: &Self,
                epsilon: Self::Epsilon,
                max_relative: Self::Epsilon,
            ) -> bool {
                let $arg1 = self;
                let $arg2 = other;
                let $arg3 = epsilon;
                let $arg4 = max_relative;
                $body
            }
        }
    };
}
