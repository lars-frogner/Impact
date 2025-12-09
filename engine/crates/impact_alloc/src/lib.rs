//! Allocation.

pub mod arena;

pub use allocator_api2;
pub use allocator_api2::alloc::Global;

pub type AVec<T, A> = allocator_api2::vec::Vec<T, A>;

pub trait Allocator: allocator_api2::alloc::Allocator + Copy {}

impl Allocator for Global {}
impl Allocator for &arena::PoolArena {}

/// Creates an [`AVec`] containing the arguments.
///
/// `avec!` allows `AVec`s to be defined with the same syntax as array expressions.
/// There are two forms of this macro:
///
/// - Create an [`AVec`] containing a given list of elements:
///
/// ```
/// use impact_alloc::avec;
/// let v = avec![1, 2, 3];
/// assert_eq!(v[0], 1);
/// assert_eq!(v[1], 2);
/// assert_eq!(v[2], 3);
/// ```
///
///
/// ```
/// use impact_alloc::{avec, Global};
/// let v = avec![in Global; 1, 2, 3];
/// assert_eq!(v[0], 1);
/// assert_eq!(v[1], 2);
/// assert_eq!(v[2], 3);
/// ```
///
/// - Create an [`AVec`] from a given element and size:
///
/// ```
/// use impact_alloc::avec;
/// let v = avec![1; 3];
/// assert_eq!(v, [1, 1, 1]);
/// ```
///
/// ```
/// use impact_alloc::{avec, Global};
/// let v = avec![in Global; 1; 3];
/// assert_eq!(v, [1, 1, 1]);
/// ```
///
/// Note that unlike array expressions this syntax supports all elements
/// which implement [`Clone`] and the number of elements doesn't have to be
/// a constant.
///
/// This will use `clone` to duplicate an expression, so one should be careful
/// using this with types having a nonstandard `Clone` implementation. For
/// example, `avec![Rc::new(1); 5]` will create a vector of five references
/// to the same boxed integer value, not five references pointing to independently
/// boxed integers.
///
/// Also, note that `avec![expr; 0]` is allowed, and produces an empty vector.
/// This will still evaluate `expr`, however, and immediately drop the resulting value, so
/// be mindful of side effects.
#[macro_export]
macro_rules! avec {
    ($($tt:tt)*) => {
        $crate::allocator_api2::vec![$($tt)*]
    };
}
