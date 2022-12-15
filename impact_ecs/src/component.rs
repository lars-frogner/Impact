//! Representation and storage of ECS components.

use bytemuck::Pod;
use std::{any::TypeId, mem};

/// Represents a component.
///
/// Components are plain data structures representing
/// a potential attribute an entity in the world can
/// have. Examples could be velocity, light source or
/// gravity.
///
/// Components can only contain "Plain Old Data", meaning
/// primitive types excluding references. The `Component`
/// trait can be derived for any type that implements
/// [`Pod`].
///
/// # Example
/// ```
/// # use impact_ecs_macros::ComponentDoctest as Component;
/// # use bytemuck::{Zeroable, Pod};
/// #
/// // Define a transform component that implements `Component`
///
/// #[repr(C)] // Required for `Pod`
/// #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// struct Transform {
///     matrix: [[f32; 4]; 4]
/// }
/// ```
pub trait Component: Pod {
    /// Returns a unique ID representing the component type.
    fn component_id() -> ComponentID {
        TypeId::of::<Self>()
    }

    /// Returns the [`ComponentByteView`] containing a reference
    /// to the raw data of the component.
    fn component_bytes(&self) -> ComponentByteView<'_>;
}

/// Represents a collection of instances of the same component
/// type.
pub trait ComponentInstances<'a, C: Component> {
    /// Returns a unique ID representing the component type.
    fn component_id() -> ComponentID {
        C::component_id()
    }

    /// Returns the number of component instances.
    fn component_count(&self) -> usize;

    /// Returns the [`ComponentByteView`] containing a reference
    /// to the raw data of the collection of components.
    fn component_bytes(&self) -> ComponentByteView<'a>;
}

/// A unique ID identifying a type implementing [`Component`].
/// It corresponds to the [`TypeId`] of the component type.
pub type ComponentID = TypeId;

/// Container that stores instances of one type of [`Component`]
/// contiguously in memory without exposing the underlying type
/// in the type signature.
///
/// # Note
/// Can also "store" zero-sized components, but without providing
/// references to stored component values since no values are
/// actually stored.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentStorage {
    component_id: ComponentID,
    component_count: usize,
    component_size: usize,
    bytes: Vec<u8>,
}

/// Container owning the bytes associated with one or more
/// components of the same type, along with the component ID,
/// count and size required to safely reconstruct the components.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentBytes {
    component_id: ComponentID,
    component_count: usize,
    component_size: usize,
    bytes: Vec<u8>,
}

/// Reference to the bytes of one or more components of the same
/// type, which also includes the component ID, count and size
/// required to safely reconstruct the components.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentByteView<'a> {
    component_id: ComponentID,
    component_count: usize,
    component_size: usize,
    bytes: &'a [u8],
}

impl ComponentStorage {
    /// Initializes a new storage for instances of the component
    /// type that the given component bytes are associated with,
    /// and stores the given bytes there.
    pub(crate) fn new_with_bytes(
        ComponentByteView {
            component_id,
            component_count,
            component_size,
            bytes,
        }: ComponentByteView<'_>,
    ) -> Self {
        Self {
            component_id,
            component_count,
            component_size,
            bytes: bytes.to_vec(),
        }
    }

    /// Returns the size of the storage in bytes.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// Returns the number of stored components.
    pub fn component_count(&self) -> usize {
        self.component_count
    }

    /// Returns a slice of all stored components.
    ///
    /// # Panics
    /// - If `C` is not the component type the storage was initialized with.
    /// - If `C` is a zero-sized type.
    pub fn slice<C: Component>(&self) -> &[C] {
        self.validate_component::<C>();
        assert_ne!(
            mem::size_of::<C>(),
            0,
            "Tried to obtain slice of zero-sized component values from storage"
        );
        bytemuck::cast_slice(&self.bytes)
    }

    /// Returns a mutable slice of all stored components.
    ///
    /// # Panics
    /// - If `C` is not the component type the storage was initialized with.
    /// - If `C` is a zero-sized type.
    pub fn slice_mut<C: Component>(&mut self) -> &mut [C] {
        self.validate_component::<C>();
        assert_ne!(
            mem::size_of::<C>(),
            0,
            "Tried to obtain slice of zero-sized component values from storage"
        );
        bytemuck::cast_slice_mut(&mut self.bytes)
    }

    /// Appends the given component to the end of the storage.
    ///
    /// # Panics
    /// If `C` is not the component type the storage was initialized with.
    pub fn push<C: Component>(&mut self, component: &C) {
        self.validate_component::<C>();
        self.bytes.extend_from_slice(bytemuck::bytes_of(component));
        self.component_count += 1;
    }

    /// Adds the given component bytes to the end of the storage.
    ///
    /// # Panics
    /// If the component ID associated with the given bytes does not
    /// correspond to the type the storage was initialized with.
    pub fn push_bytes(
        &mut self,
        ComponentByteView {
            component_id,
            component_count,
            component_size: _,
            bytes,
        }: ComponentByteView<'_>,
    ) {
        self.validate_component_id(component_id);
        self.bytes.extend_from_slice(bytes);
        self.component_count += component_count;
    }

    /// Removes the component at the given index and makes the
    /// last component take its place (unless the one to remove
    /// is the last one).
    ///
    /// # Returns
    ///  The removed component.
    ///
    /// # Panics
    /// - If `C` is not the component type the storage was initialized with.
    /// - If `idx` is outside the bounds of the storage.
    pub fn swap_remove<C: Component>(&mut self, idx: usize) -> C {
        self.validate_component::<C>();

        let removed_component = if self.component_size > 0 {
            let components = self.slice_mut::<C>();
            let n_components = components.len();
            assert!(idx < n_components, "Index for component out of bounds");

            let removed_component = components[idx];

            // Swap with last component unless the component to
            // remove is the last one
            let last_component_idx = n_components - 1;
            if idx < last_component_idx {
                components.swap(idx, last_component_idx);
            }

            // Remove last component (this must be done on the raw byte `Vec`)
            self.bytes.truncate(self.bytes.len() - mem::size_of::<C>());

            removed_component
        } else {
            // If the component is zero-sized, return the only possible "value"
            // (calling `zeroed` is just a way of getting a value instead of a type)
            C::zeroed()
        };

        self.component_count = self.component_count.checked_sub(1).unwrap();

        removed_component
    }

    /// Type erased version of [`Self::swap_remove`].
    ///
    /// # Note
    /// `idx` still refers to the whole component, not its byte boundary.
    ///
    /// # Returns
    ///  The removed component bytes.
    ///
    /// # Panics
    /// If `idx` is outside the bounds of the storage.
    pub fn swap_remove_bytes(&mut self, idx: usize) -> ComponentBytes {
        assert!(
            idx < self.component_count(),
            "Index for component out of bounds"
        );

        let removed_component_data = if self.component_size > 0 {
            let component_to_remove_start = idx.checked_mul(self.component_size).unwrap();
            let data_size = self.bytes.len();

            let removed_component_data = ComponentBytes::new_for_single_instance(
                self.component_id,
                self.component_size,
                self.bytes
                    [component_to_remove_start..component_to_remove_start + self.component_size]
                    .to_vec(),
            );

            // Copy over with last component unless the component to
            // remove is the last one
            let last_component_start = data_size - self.component_size;
            if component_to_remove_start < last_component_start {
                unsafe {
                    // Pointer to beginning of last component
                    let src_ptr = self.bytes.as_ptr().add(last_component_start);

                    // Mutable pointer to beginning of component to remove
                    let dst_ptr = self.bytes.as_mut_ptr().add(component_to_remove_start);

                    // Copy last component over component to remove
                    std::ptr::copy_nonoverlapping::<u8>(src_ptr, dst_ptr, self.component_size);
                }
            }

            // Remove last component (this must be done on the raw byte `Vec`)
            self.bytes.truncate(last_component_start);

            removed_component_data
        } else {
            ComponentBytes::new_for_single_zero_sized_instance(self.component_id)
        };

        self.component_count = self.component_count.checked_sub(1).unwrap();

        removed_component_data
    }

    fn validate_component<C: Component>(&self) {
        self.validate_component_id(C::component_id());
    }

    fn validate_component_id(&self, component_id: ComponentID) {
        assert!(
            component_id == self.component_id,
            "Tried to use component storage with invalid component"
        );
    }
}

impl ComponentBytes {
    /// Creates a new container for the given bytes for a component
    /// with the given ID, count and size.
    pub(crate) fn new(
        component_id: ComponentID,
        component_count: usize,
        component_size: usize,
        bytes: Vec<u8>,
    ) -> Self {
        assert_eq!(
            component_count.checked_mul(component_size).unwrap(),
            bytes.len()
        );
        Self {
            component_id,
            component_count,
            component_size,
            bytes,
        }
    }

    /// Creates a new container for the given bytes for a single
    /// instance of the component with the given ID, count and size.
    pub(crate) fn new_for_single_instance(
        component_id: ComponentID,
        component_size: usize,
        bytes: Vec<u8>,
    ) -> Self {
        Self::new(component_id, 1, component_size, bytes)
    }

    /// Creates a new container for a single instance of a zero-sized
    /// component.
    pub(crate) fn new_for_single_zero_sized_instance(component_id: ComponentID) -> Self {
        Self::new_for_single_instance(component_id, 0, Vec::new())
    }

    /// Returns the ID of the component type these bytes represent.
    pub fn component_id(&self) -> ComponentID {
        self.component_id
    }

    /// Returns the size of the component type these bytes represent.
    pub fn component_size(&self) -> usize {
        self.component_size
    }

    /// Returns the number of component instances these bytes represent.
    pub fn component_count(&self) -> usize {
        self.component_count
    }

    /// Returns a [`ComponentByteView`] referencing the component
    /// bytes.
    pub fn as_ref(&self) -> ComponentByteView<'_> {
        ComponentByteView {
            component_id: self.component_id(),
            component_count: self.component_count(),
            component_size: self.component_size(),
            bytes: &self.bytes,
        }
    }
}

impl<'a> ComponentByteView<'a> {
    /// Creates a new view to the given bytes for a component
    /// with the given ID, count and size.
    pub fn new(
        component_id: ComponentID,
        component_count: usize,
        component_size: usize,
        bytes: &'a [u8],
    ) -> Self {
        assert_eq!(
            component_count.checked_mul(component_size).unwrap(),
            bytes.len()
        );
        Self {
            component_id,
            component_count,
            component_size,
            bytes,
        }
    }

    /// Creates a new view to the given bytes for a single instance
    /// of the component with the given ID, count and size.
    pub fn new_for_single_instance(
        component_id: ComponentID,
        component_size: usize,
        bytes: &'a [u8],
    ) -> Self {
        Self::new(component_id, 1, component_size, bytes)
    }

    /// Returns the ID of the type of the components whose bytes
    /// this reference points to.
    pub fn component_id(&self) -> ComponentID {
        self.component_id
    }

    /// Returns the size of the type of the components whose bytes
    /// this reference points to.
    pub fn component_size(&self) -> usize {
        self.component_size
    }

    /// Returns the number of component instances this reference points to.
    pub fn component_count(&self) -> usize {
        self.component_count
    }

    /// Creates a [`ComponentBytes`] holding a copy of the referenced
    /// component bytes.
    pub fn to_owned(&self) -> ComponentBytes {
        ComponentBytes {
            component_id: self.component_id(),
            component_count: self.component_count(),
            component_size: self.component_size(),
            bytes: self.bytes.to_vec(),
        }
    }
}

impl<'a, C: Component> ComponentInstances<'a, C> for &'a [C] {
    fn component_count(&self) -> usize {
        self.len()
    }

    fn component_bytes(&self) -> ComponentByteView<'a> {
        ComponentByteView::new(
            Self::component_id(),
            self.len(),
            mem::size_of::<C>(),
            bytemuck::cast_slice(self),
        )
    }
}

impl<'a, const N: usize, C: Component> ComponentInstances<'a, C> for &'a [C; N] {
    fn component_count(&self) -> usize {
        self.len()
    }

    fn component_bytes(&self) -> ComponentByteView<'a> {
        ComponentByteView::new(
            Self::component_id(),
            self.len(),
            mem::size_of::<C>(),
            bytemuck::cast_slice(*self),
        )
    }
}

#[cfg(test)]
mod test {
    use super::{super::Component, *};
    use bytemuck::Zeroable;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Marked;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Byte(u8);

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Rectangle {
        center: [f32; 2],
        dimensions: [f32; 2],
    }

    const RECT_1: Rectangle = Rectangle {
        center: [2.5, 2.0],
        dimensions: [12.3, 8.9],
    };

    const RECT_2: Rectangle = Rectangle {
        center: [-11.1, 0.01],
        dimensions: [1.2, 33.0],
    };

    const RECT_3: Rectangle = Rectangle {
        center: [12.1, -0.1],
        dimensions: [2.1, 3.0],
    };

    #[test]
    fn referencing_component_data_works() {
        let component = Byte(42);
        let data = component.component_bytes();
        assert_eq!(data.component_count(), 1);
        assert_eq!(data.component_size(), mem::size_of::<Byte>());
        assert_eq!(data.bytes.len(), 1);
        assert_eq!(data.bytes[0], 42);
    }

    #[test]
    fn creating_component_data_for_zero_sized_component_works() {
        let component = Marked;
        let data = component.component_bytes();
        assert_eq!(data.component_count(), 1);
        assert_eq!(data.component_size(), 0);
        assert_eq!(data.bytes.len(), 0);
    }

    #[test]
    fn storage_initialization_works() {
        let storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.size(), mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1]);
    }

    #[test]
    #[should_panic]
    fn requesting_slice_of_wrong_component_from_storage_fails() {
        let storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.slice::<Byte>();
    }

    #[test]
    #[should_panic]
    fn requesting_mutable_slice_of_wrong_component_from_storage_fails() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.slice_mut::<Byte>();
    }

    #[test]
    #[should_panic]
    fn requesting_slice_of_zero_sized_component_from_storage_fails() {
        let storage = ComponentStorage::new_with_bytes(Marked.component_bytes());
        storage.slice::<Marked>();
    }

    #[test]
    #[should_panic]
    fn requesting_mutable_slice_of_zero_sized_component_from_storage_fails() {
        let mut storage = ComponentStorage::new_with_bytes(Marked.component_bytes());
        storage.slice_mut::<Marked>();
    }

    #[test]
    fn modifying_stored_component_works() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.slice_mut::<Rectangle>()[0] = RECT_2;
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2]);
    }

    #[test]
    #[should_panic]
    fn pushing_different_component_to_storage_fails() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.push(&Byte(42));
    }

    #[test]
    fn pushing_component_to_storage_works() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.push(&RECT_2);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.size(), 2 * mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_2]);
    }

    #[test]
    fn pushing_zero_sized_component_to_storage_works() {
        let mut storage = ComponentStorage::new_with_bytes(Marked.component_bytes());
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.size(), 0);
    }

    #[test]
    #[should_panic]
    fn pushing_different_component_bytes_to_storage_fails() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.push_bytes(Byte(42).component_bytes());
    }

    #[test]
    fn pushing_component_bytes_to_storage_works() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.push_bytes(RECT_2.component_bytes());
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.size(), 2 * mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_2]);
    }

    #[test]
    fn pushing_zero_sized_component_bytes_to_storage_works() {
        let mut storage = ComponentStorage::new_with_bytes(Marked.component_bytes());
        storage.push_bytes(Marked.component_bytes());
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.size(), 0);
    }

    #[test]
    #[should_panic]
    fn removing_different_component_from_storage_fails() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.swap_remove::<Byte>(0);
    }

    #[test]
    #[should_panic]
    fn removing_component_from_storage_with_invalid_idx_fails() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        storage.swap_remove::<Rectangle>(1);
    }

    #[test]
    fn swap_removing_component_from_storage_works() {
        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        assert_eq!(storage.swap_remove::<Rectangle>(0), RECT_1);
        assert_eq!(storage.slice::<Rectangle>(), &[]);

        storage.push(&RECT_1);
        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove::<Rectangle>(0), RECT_1);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2]);
        assert_eq!(storage.swap_remove::<Rectangle>(0), RECT_2);
        assert_eq!(storage.slice::<Rectangle>(), &[]);

        storage.push(&RECT_1);
        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove::<Rectangle>(1), RECT_2);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1]);

        storage.push(&RECT_2);
        storage.push(&RECT_3);
        assert_eq!(storage.swap_remove::<Rectangle>(1), RECT_2);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_3]);

        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove::<Rectangle>(0), RECT_1);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2, RECT_3]);
    }

    #[test]
    fn swap_removing_zero_sized_component_from_storage_works() {
        let mut storage = ComponentStorage::new_with_bytes(Marked.component_bytes());
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.swap_remove::<Marked>(0), Marked);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.swap_remove::<Marked>(0), Marked);
        assert_eq!(storage.component_count(), 0);

        storage.push(&Marked);
        storage.push(&Marked);
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 3);
        assert_eq!(storage.swap_remove::<Marked>(1), Marked);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.swap_remove::<Marked>(1), Marked);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.swap_remove::<Marked>(0), Marked);
        assert_eq!(storage.component_count(), 0);
    }

    #[test]
    fn swap_removing_component_bytes_from_storage_works() {
        let rect_1_bytes = RECT_1.component_bytes().to_owned();
        let rect_2_bytes = RECT_2.component_bytes().to_owned();

        let mut storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
        assert_eq!(storage.swap_remove_bytes(0), rect_1_bytes);
        assert_eq!(storage.slice::<Rectangle>(), &[]);

        storage.push(&RECT_1);
        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove_bytes(0), rect_1_bytes);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2]);
        assert_eq!(storage.swap_remove_bytes(0), rect_2_bytes);
        assert_eq!(storage.slice::<Rectangle>(), &[]);

        storage.push(&RECT_1);
        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove_bytes(1), rect_2_bytes);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1]);

        storage.push(&RECT_2);
        storage.push(&RECT_3);
        assert_eq!(storage.swap_remove_bytes(1), rect_2_bytes);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_3]);

        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove_bytes(0), rect_1_bytes);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2, RECT_3]);
    }

    #[test]
    fn swap_removing_zero_sized_component_bytes_from_storage_works() {
        let marked_bytes = Marked.component_bytes().to_owned();

        let mut storage = ComponentStorage::new_with_bytes(Marked.component_bytes());
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.swap_remove_bytes(0), marked_bytes);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.swap_remove_bytes(0), marked_bytes);
        assert_eq!(storage.component_count(), 0);

        storage.push(&Marked);
        storage.push(&Marked);
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 3);
        assert_eq!(storage.swap_remove_bytes(1), marked_bytes);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.swap_remove_bytes(1), marked_bytes);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.swap_remove_bytes(0), marked_bytes);
        assert_eq!(storage.component_count(), 0);
    }
}
