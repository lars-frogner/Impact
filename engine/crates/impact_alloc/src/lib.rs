//! Allocation.

pub mod arena;

pub use allocator_api2::alloc::Global;
pub use allocator_api2::vec;

pub type AVec<T, A> = allocator_api2::vec::Vec<T, A>;

pub trait Allocator: allocator_api2::alloc::Allocator + Copy {}

impl Allocator for &arena::Arena {}
impl Allocator for Global {}
