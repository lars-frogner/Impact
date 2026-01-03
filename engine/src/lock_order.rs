//! Optional system to enforce that engine resource locks are accessed in a
//! consistent order.

pub mod resources;

use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Trait for types that have an associated lock order. Types with lower order
/// values must be acquired before types with higher values.
pub trait LockableResource {
    /// The lock order for this resource type. Lower values must be acquired
    /// first.
    const LOCK_ORDER: LockOrder;

    fn resource_name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Trait for acquiring locks in a type-safe manner with automatic ordering
/// validation.
pub trait OrderedRwLock<T> {
    fn oread(&self) -> OrderedLockGuard<RwLockReadGuard<'_, T>>;
    fn owrite(&self) -> OrderedLockGuard<RwLockWriteGuard<'_, T>>;
}

/// Trait for acquiring mutex locks in a type-safe manner with automatic
/// ordering validation.
pub trait OrderedMutex<T> {
    fn olock(&self) -> OrderedLockGuard<MutexGuard<'_, T>>;
}

/// Lock order value - lower values must be acquired first.
pub type LockOrder = u16;

pub use inner::*;

#[cfg(feature = "checked_lock_order")]
pub fn set_panic_on_violation(enabled: bool) {
    LOCK_VALIDATOR.set_panic_on_violation(enabled);
}
#[cfg(not(feature = "checked_lock_order"))]
pub fn set_panic_on_violation(_enabled: bool) {}

#[cfg(feature = "checked_lock_order")]
#[macro_use]
mod inner {
    use super::*;
    use impact_alloc::Global;
    use impact_containers::HashMap;
    use std::{
        backtrace::Backtrace,
        mem::ManuallyDrop,
        ptr,
        sync::{
            LazyLock,
            atomic::{AtomicBool, Ordering},
        },
        thread::{self, ThreadId},
    };

    /// Macro to implement [`LockableResource`] and automatically register the
    /// type.
    #[macro_export]
    macro_rules! declare_lockable_resource {
        ($type:ty, $order:expr) => {
            impl $crate::lock_order::LockableResource for $type {
                const LOCK_ORDER: $crate::lock_order::LockOrder = $order;
            }

            inventory::submit! {
                $crate::lock_order::LockOrderDescriptor {
                    order: $order,
                    name: stringify!($type),
                }
            }
        };
    }

    /// A descriptor for a lockable resource that gets automatically registered.
    #[derive(Debug, Clone, Copy)]
    pub struct LockOrderDescriptor {
        /// The lock order for this resource type.
        pub order: LockOrder,
        /// The name of the resource type.
        pub name: &'static str,
    }

    inventory::collect!(LockOrderDescriptor);

    /// RAII guard that automatically records lock release.
    #[derive(Debug)]
    pub struct OrderedLockGuard<G> {
        guard: G,
        lock_order: LockOrder,
        resource_name: &'static str,
    }

    #[derive(Debug)]
    pub struct LockOrderValidator {
        thread_locks: Mutex<HashMap<ThreadId, Vec<LockInfo>>>,
        panic_on_violation: AtomicBool,
    }

    pub static LOCK_VALIDATOR: LazyLock<LockOrderValidator> =
        LazyLock::new(LockOrderValidator::new);

    #[derive(Debug, Clone)]
    struct LockInfo {
        lock_order: LockOrder,
        resource_name: &'static str,
        lock_type: LockType,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum LockType {
        Read,
        Write,
        Exclusive,
    }

    impl<'a, T: LockableResource> OrderedLockGuard<RwLockWriteGuard<'a, T>> {
        pub fn downgrade(self) -> OrderedLockGuard<RwLockReadGuard<'a, T>> {
            let this = ManuallyDrop::new(self);

            // SAFETY: we won't use `this` again; `ManuallyDrop` prevents its Drop.
            let write_guard = unsafe { ptr::read(&this.guard) };

            let lock_order = this.lock_order;
            let resource_name = this.resource_name;

            LOCK_VALIDATOR.record_release(lock_order, resource_name);
            LOCK_VALIDATOR.validate_and_record(lock_order, resource_name, LockType::Read);

            OrderedLockGuard {
                guard: RwLockWriteGuard::downgrade(write_guard),
                lock_order,
                resource_name,
            }
        }
    }

    impl<G> std::ops::Deref for OrderedLockGuard<G> {
        type Target = G;

        fn deref(&self) -> &Self::Target {
            &self.guard
        }
    }

    impl<G> std::ops::DerefMut for OrderedLockGuard<G> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.guard
        }
    }

    impl<G> Drop for OrderedLockGuard<G> {
        fn drop(&mut self) {
            LOCK_VALIDATOR.record_release(self.lock_order, self.resource_name);
        }
    }

    impl<T: LockableResource> OrderedRwLock<T> for RwLock<T> {
        fn oread(&self) -> OrderedLockGuard<RwLockReadGuard<'_, T>> {
            let lock_order = T::LOCK_ORDER;
            let resource_name = T::resource_name();

            LOCK_VALIDATOR.validate_and_record(lock_order, resource_name, LockType::Read);

            OrderedLockGuard {
                guard: self.read(),
                lock_order,
                resource_name,
            }
        }

        fn owrite(&self) -> OrderedLockGuard<RwLockWriteGuard<'_, T>> {
            let lock_order = T::LOCK_ORDER;
            let resource_name = T::resource_name();

            LOCK_VALIDATOR.validate_and_record(lock_order, resource_name, LockType::Write);

            OrderedLockGuard {
                guard: self.write(),
                lock_order,
                resource_name,
            }
        }
    }

    impl<T: LockableResource> OrderedMutex<T> for Mutex<T> {
        fn olock(&self) -> OrderedLockGuard<MutexGuard<'_, T>> {
            let lock_order = T::LOCK_ORDER;
            let resource_name = T::resource_name();

            LOCK_VALIDATOR.validate_and_record(lock_order, resource_name, LockType::Exclusive);

            OrderedLockGuard {
                guard: self.lock(),
                lock_order,
                resource_name,
            }
        }
    }

    impl LockOrderValidator {
        fn new() -> Self {
            validate_declared_lockable_resources();
            Self {
                thread_locks: Mutex::new(HashMap::default()),
                panic_on_violation: AtomicBool::new(true),
            }
        }

        pub fn set_panic_on_violation(&self, enabled: bool) {
            self.panic_on_violation.store(enabled, Ordering::Release);
        }

        fn validate_and_record(
            &self,
            lock_order: LockOrder,
            resource_name: &'static str,
            lock_type: LockType,
        ) {
            let thread_id = thread::current().id();
            let lock_info = LockInfo {
                lock_order,
                resource_name,
                lock_type,
            };

            let mut thread_locks = self.thread_locks.lock();
            let locks_for_thread = thread_locks.entry(thread_id).or_default();

            // Check ordering violation - we should acquire locks in order (lower values first)
            for held_lock in locks_for_thread.iter() {
                if lock_order < held_lock.lock_order {
                    let backtrace = Backtrace::capture();

                    log::error!(
                        "Lock ordering violation: attempting to acquire {resource_name} (order {lock_order}) while holding {} (order {}).\n\
                         Current locks held (in acquisition order): [{}]\n
                         {backtrace}",
                        held_lock.resource_name,
                        held_lock.lock_order,
                        locks_for_thread
                            .iter()
                            .map(|lock| format!(
                                "{}({:?}, {})",
                                lock.resource_name, lock.lock_type, lock.lock_order
                            ))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    if self.panic_on_violation.load(Ordering::Acquire) {
                        panic!("Lock order violated");
                    }
                }
            }

            locks_for_thread.push(lock_info);
        }

        fn record_release(&self, lock_order: LockOrder, resource_name: &'static str) {
            let thread_id = thread::current().id();
            let mut thread_locks = self.thread_locks.lock();

            if let Some(locks_for_thread) = thread_locks.get_mut(&thread_id) {
                // Find and remove the most recent lock for this resource
                if let Some(pos) = locks_for_thread.iter().rposition(|lock| {
                    lock.lock_order == lock_order && lock.resource_name == resource_name
                }) {
                    locks_for_thread.remove(pos);
                }

                if locks_for_thread.is_empty() {
                    thread_locks.remove(&thread_id);
                }
            }
        }
    }

    /// Checks that each registered lockable resource has a unique lock order.
    fn validate_declared_lockable_resources() {
        let mut orders = HashMap::<_, _, Global>::default();
        for descriptor in inventory::iter::<LockOrderDescriptor> {
            if let Some(existing_name) = orders.get(&descriptor.order) {
                panic!(
                    "Lock order conflict detected: both '{}' and '{}' have lock order {}",
                    existing_name, descriptor.name, descriptor.order
                );
            }
            orders.insert(descriptor.order, descriptor.name);
        }
    }
}

#[cfg(not(feature = "checked_lock_order"))]
#[macro_use]
mod inner {
    use super::*;

    /// Macro to implement [`LockableResource`].
    #[macro_export]
    macro_rules! declare_lockable_resource {
        ($type:ty, $order:expr) => {
            impl $crate::lock_order::LockableResource for $type {
                const LOCK_ORDER: $crate::lock_order::LockOrder = $order;
            }
        };
    }

    #[derive(Debug)]
    pub struct OrderedLockGuard<G>(G);

    impl<'a, T: LockableResource> OrderedLockGuard<RwLockWriteGuard<'a, T>> {
        pub fn downgrade(self) -> OrderedLockGuard<RwLockReadGuard<'a, T>> {
            OrderedLockGuard(RwLockWriteGuard::downgrade(self.0))
        }
    }

    impl<G> std::ops::Deref for OrderedLockGuard<G> {
        type Target = G;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<G> std::ops::DerefMut for OrderedLockGuard<G> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T: LockableResource> OrderedRwLock<T> for RwLock<T> {
        fn oread(&self) -> OrderedLockGuard<RwLockReadGuard<'_, T>> {
            OrderedLockGuard(self.read())
        }

        fn owrite(&self) -> OrderedLockGuard<RwLockWriteGuard<'_, T>> {
            OrderedLockGuard(self.write())
        }
    }

    impl<T: LockableResource> OrderedMutex<T> for Mutex<T> {
        fn olock(&self) -> OrderedLockGuard<MutexGuard<'_, T>> {
            OrderedLockGuard(self.lock())
        }
    }
}

#[cfg(feature = "checked_lock_order")]
#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::{Mutex, RwLock};

    struct TestResourceA;
    struct TestResourceB;

    declare_lockable_resource!(TestResourceA, 9000);
    declare_lockable_resource!(TestResourceB, 9010);

    #[test]
    fn test_correct_lock_ordering() {
        set_panic_on_violation(true);

        let lock_a = RwLock::new(TestResourceA);
        let lock_b = RwLock::new(TestResourceB);

        // This should succeed - acquiring locks in order (10 before 20)
        let _guard_a = lock_a.oread();
        let _guard_b = lock_b.oread();
    }

    #[test]
    #[should_panic(expected = "Lock order violated")]
    fn test_lock_ordering_violation() {
        set_panic_on_violation(true);

        // Use different types to avoid conflicts with other tests
        struct TestResourceX;
        struct TestResourceY;

        impl LockableResource for TestResourceX {
            const LOCK_ORDER: LockOrder = 9200;
        }

        impl LockableResource for TestResourceY {
            const LOCK_ORDER: LockOrder = 9210;
        }

        let lock_x = RwLock::new(TestResourceX);
        let lock_y = RwLock::new(TestResourceY);

        // Acquire Y (order 9210) first
        let _guard_y = lock_y.oread();

        // This should panic - trying to acquire X (order 9200) after Y (order 9210)
        let _guard_x = lock_x.oread();
    }

    #[test]
    fn test_multiple_read_locks_allowed() {
        set_panic_on_violation(true);

        let lock = RwLock::new(TestResourceA);

        let _read_guard1 = lock.oread();
        let _read_guard2 = lock.oread(); // This should be fine
    }

    #[test]
    fn test_mutex_ordering() {
        set_panic_on_violation(true);

        let mutex_a = Mutex::new(TestResourceA);
        let mutex_b = Mutex::new(TestResourceB);

        let _guard_a = mutex_a.olock();
        let _guard_b = mutex_b.olock();
    }
}
