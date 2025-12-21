//! Utility macros.

/// Creates a [`StringHash32`](crate::hash::StringHash32) for
/// the given string.
#[macro_export]
macro_rules! hash32 {
    ($string:literal) => {
        $crate::hash::StringHash32::new_with_hash($string, $crate::hash::Hash32::from_str($string))
    };
    ($string:expr) => {
        $crate::hash::StringHash32::new($string)
    };
}

/// Creates a [`StringHash64`](crate::hash::StringHash64) for
/// the given string.
#[macro_export]
macro_rules! hash64 {
    ($string:literal) => {
        $crate::hash::StringHash64::new_with_hash($string, $crate::hash::Hash64::from_str($string))
    };
    ($string:expr) => {
        $crate::hash::StringHash64::new($string)
    };
}

/// Defines a new type with the given name that is a wrapper
/// around a [`StringHash32`](crate::hash::StringHash32).
#[macro_export]
macro_rules! stringhash32_newtype {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Zeroable, bytemuck::Pod)]
        $($pub)? struct $name($($pub)? $crate::hash::StringHash32);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

/// Defines a new type with the given name that is a wrapper
/// around a [`StringHash64`](crate::hash::StringHash64).
#[macro_export]
macro_rules! stringhash64_newtype {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Zeroable, bytemuck::Pod)]
        $($pub)? struct $name($($pub)? $crate::hash::StringHash64);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
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
