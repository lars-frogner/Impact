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
/// trait is automatically implemented for any type that
/// implements [`Pod`].
///
/// # Example
/// ```
/// // Define a transform component
/// # use bytemuck::{Zeroable, Pod};
/// #[repr(C)] // Required for `Pod`
/// #[derive(Clone, Copy, Zeroable, Pod)]
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
    fn component_bytes(&self) -> ComponentByteView;
}

/// A unique ID identifying a type implementing [`Component`].
/// It corresponds to the [`TypeId`] of the component type.
pub type ComponentID = TypeId;

/// Container that stores instances of one type of [`Component`]
/// contiguously in memory without exposing the underlying type
/// in the type signature.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentStorage {
    component_id: ComponentID,
    component_size: usize,
    bytes: Vec<u8>,
}

/// Container owning the bytes associated with a component,
/// along with the component ID required to safely reconstruct
/// the component.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentBytes {
    component_id: ComponentID,
    bytes: Vec<u8>,
}

/// Reference to the bytes of a component, which also includes
/// the component ID required to safely reconstruct the component.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentByteView<'a> {
    component_id: ComponentID,
    bytes: &'a [u8],
}

impl ComponentStorage {
    /// Initializes a new storage for instances of the component
    /// type that the given component bytes are associated with, and
    /// stores the given bytes there.
    pub fn new_with_bytes(
        ComponentByteView {
            component_id,
            bytes,
        }: ComponentByteView,
    ) -> Self {
        Self {
            component_id,
            component_size: bytes.len(),
            bytes: bytes.to_vec(),
        }
    }

    /// Returns the size of the storage in bytes.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// Returns the number of stored components.
    ///
    /// # Panics
    /// If `C` is not the component type the storage was initialized with.
    pub fn n_components<C: Component>(&self) -> usize {
        self.slice::<C>().len()
    }

    /// Returns a slice of all stored components.
    ///
    /// # Panics
    /// If `C` is not the component type the storage was initialized with.
    pub fn slice<C: Component>(&self) -> &[C] {
        self.validate_component::<C>();
        bytemuck::cast_slice(&self.bytes)
    }

    /// Returns a mutable slice of all stored components.
    ///
    /// # Panics
    /// If `C` is not the component type the storage was initialized with.
    pub fn slice_mut<C: Component>(&mut self) -> &mut [C] {
        self.validate_component::<C>();
        bytemuck::cast_slice_mut(&mut self.bytes)
    }

    /// Appends the given component to the end of the storage.
    ///
    /// # Panics
    /// If `C` is not the component type the storage was initialized with.
    pub fn push<C: Component>(&mut self, component: &C) {
        self.validate_component::<C>();
        self.bytes.extend_from_slice(bytemuck::bytes_of(component));
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
            bytes,
        }: ComponentByteView,
    ) {
        self.validate_component_id(component_id);
        self.bytes.extend_from_slice(bytes);
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
        let component_to_remove_start = idx.checked_mul(self.component_size).unwrap();
        let data_size = self.bytes.len();
        assert!(
            component_to_remove_start < data_size,
            "Index for component out of bounds"
        );

        let removed_component_data = ComponentBytes {
            component_id: self.component_id,
            bytes: self.bytes
                [component_to_remove_start..component_to_remove_start + self.component_size]
                .to_vec(),
        };

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

// Implement `Component` for all types implementing `Pod`
impl<C: Pod> Component for C {
    fn component_bytes(&self) -> ComponentByteView {
        ComponentByteView {
            component_id: Self::component_id(),
            bytes: bytemuck::bytes_of(self),
        }
    }
}

impl ComponentBytes {
    /// Returns the ID of the component these bytes represent.
    pub fn component_id(&self) -> ComponentID {
        self.component_id
    }

    /// Returns a [`ComponentByteView`] referencing the component
    /// bytes.
    pub fn as_ref(&self) -> ComponentByteView {
        ComponentByteView {
            component_id: self.component_id(),
            bytes: &self.bytes,
        }
    }
}

impl<'a> ComponentByteView<'a> {
    /// Returns the ID of the component whose bytes this reference
    /// points to.
    pub fn component_id(&self) -> ComponentID {
        self.component_id
    }

    /// Creates a [`ComponentBytes`] holding a copy of the referenced
    /// component bytes.
    pub fn to_owned(&self) -> ComponentBytes {
        ComponentBytes {
            component_id: self.component_id(),
            bytes: self.bytes.to_vec(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytemuck::Zeroable;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    struct Byte(u8);

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
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
        assert_eq!(data.bytes.len(), 1);
        assert_eq!(data.bytes[0], 42);
    }

    #[test]
    fn storage_initialization_works() {
        let storage = ComponentStorage::new_with_bytes(RECT_1.component_bytes());
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
        assert_eq!(storage.size(), 2 * mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_2]);
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
        assert_eq!(storage.size(), 2 * mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_2]);
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
}
