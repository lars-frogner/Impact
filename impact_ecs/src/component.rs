//! Representation and storage of ECS components.

use bytemuck::{Pod, Zeroable};
use impact_utils::{AlignedByteVec, Alignment};
use std::{mem, ops::Deref};

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
    const COMPONENT_ID: ComponentID;

    /// Returns a unique ID representing the component type.
    fn component_id() -> ComponentID {
        Self::COMPONENT_ID
    }
}

/// Represents a temporary [`Component`] whose purpose is to initialize an
/// entity and which will not persist after entity creation.
pub trait SetupComponent: Component {}

/// Represents a collection of instances of the same component
/// type.
pub trait ComponentArray: Clone {
    /// Returns a unique ID representing the component type.
    fn component_id(&self) -> ComponentID;

    /// Returns the number of component instances.
    fn component_count(&self) -> usize;

    /// Returns the size of a single component instance in bytes.
    fn component_size(&self) -> usize;

    /// Returns the alignment of the component type.
    fn component_align(&self) -> Alignment;

    /// Returns a type erased view of the component instances.
    fn view(&self) -> ComponentView<'_>;

    /// Turns the array into a [`ComponentStorage`] storing the
    /// same data.
    fn into_storage(self) -> ComponentStorage;
}

/// Represents a view into a collection of instances of the same
/// component type, with the lifetime of the view decoupled from
/// the lifetime of the referenced data.
pub trait ComponentSlice<'a>: ComponentArray {
    /// Returns the slice of bytes representing all the component
    /// instances.
    fn component_bytes(&self) -> &'a [u8];

    /// Returns a slice of all the component instances.
    ///
    /// # Panics
    /// - If `C` is not the stored component type.
    /// - If `C` is a zero-sized type.
    fn component_instances<C: Component>(&self) -> &'a [C];

    /// Returns a type erased view of the component instances.
    ///
    /// This method differs from [`ComponentArray::view`] in
    /// that the [`ComponentView`] returned here may outlive the
    /// called object.
    fn persistent_view(&self) -> ComponentView<'a> {
        ComponentView::new(
            self.component_id(),
            self.component_count(),
            self.component_size(),
            self.component_align(),
            self.component_bytes(),
        )
    }
}

/// A unique ID identifying a type implementing [`Component`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ComponentID(u64);

/// A descriptor for a [`Component`]. Component types can register themselves
/// in a distributed registry by evoking [`inventory::submit!`] on a static
/// `ComponentDescriptor` value.
#[derive(Debug)]
pub struct ComponentDescriptor {
    /// The ID of the component.
    pub id: ComponentID,
    /// The name of the component.
    pub name: &'static str,
    /// The category of the component.
    pub category: ComponentCategory,
}

inventory::collect!(ComponentDescriptor);

/// The category of a component.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ComponentCategory {
    /// A persistent component whose current state is always reflected in the
    /// world.
    Standard,
    /// A helper component used for creating entities, which is no longer
    /// present in the entity after it has been created.
    Setup,
}

/// Container that stores instances of one type of [`Component`]
/// contiguously in memory without exposing the underlying type
/// in the type signature.
///
/// # Note
/// Can also "store" zero-sized components, but without providing
/// references to stored component values since no values are
/// actually stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentStorage {
    component_id: ComponentID,
    component_count: usize,
    component_size: usize,
    bytes: AlignedByteVec,
}

/// Reference to the bytes of one or more components of the same
/// type, which also includes the component ID, count, size and
/// alignment required to safely reconstruct the components.
#[derive(Clone, Debug)]
pub struct ComponentView<'a> {
    component_id: ComponentID,
    component_count: usize,
    component_size: usize,
    component_align: Alignment,
    bytes: &'a [u8],
}

/// Represents a single instance of a component type.
pub trait ComponentInstance: ComponentArray {
    /// Returns a type erased view of the component instance.
    fn single_instance_view(&self) -> SingleInstance<ComponentView<'_>>;
}

/// Represents a collection of component instances where the number of instances
/// may be one. Required for a type to be wrappable in a [`SingleInstance`].
pub trait CanHaveSingleInstance {
    /// Returns the number of component instances in the collection.
    fn instance_count(&self) -> usize;
}

/// Wrapper for types holding component instances that guarantees that the
/// wrapped container only holds component data for a single instance.
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleInstance<T> {
    container: T,
}

impl ComponentID {
    pub const fn hashed_from_str(input: &str) -> Self {
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(input);
        Self(if hash == 0 { 1 } else { hash })
    }

    pub(crate) const fn dummy() -> Self {
        Self(0)
    }
}

// We can treat a reference to a component as a component array
// and slice with a single instance
impl<C: Component> ComponentArray for &C {
    fn component_id(&self) -> ComponentID {
        C::component_id()
    }

    fn component_count(&self) -> usize {
        1
    }

    fn component_size(&self) -> usize {
        mem::size_of::<C>()
    }

    fn component_align(&self) -> Alignment {
        Alignment::of::<C>()
    }

    fn view(&self) -> ComponentView<'_> {
        ComponentView::new_for_single_instance(
            C::component_id(),
            ::std::mem::size_of::<C>(),
            Alignment::of::<C>(),
            self.component_bytes(),
        )
        .into_inner()
    }

    fn into_storage(self) -> ComponentStorage {
        ComponentStorage::from_view(self.view())
    }
}

impl<'a, C: Component> ComponentSlice<'a> for &'a C {
    fn component_bytes(&self) -> &'a [u8] {
        bytemuck::bytes_of(*self)
    }

    fn component_instances<C2: Component>(&self) -> &'a [C2] {
        self.persistent_view().component_instances()
    }
}

impl<C: Component> ComponentInstance for &C {
    fn single_instance_view(&self) -> SingleInstance<ComponentView<'_>> {
        SingleInstance::new_unchecked(self.view())
    }
}

impl ComponentStorage {
    /// Initializes a new storage for component instances with
    /// the given ID and size, and stores the given bytes representing
    /// a set of component instances with the given count.
    fn new(
        component_id: ComponentID,
        component_count: usize,
        component_size: usize,
        bytes: AlignedByteVec,
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

    /// Initializes a new storage for component instances with
    /// the given ID and size, and stores the given bytes representing
    /// a single component instance.
    fn new_for_single_instance(
        component_id: ComponentID,
        component_size: usize,
        bytes: AlignedByteVec,
    ) -> SingleInstance<Self> {
        SingleInstance::new_unchecked(Self::new(component_id, 1, component_size, bytes))
    }

    /// Initializes a new storage for a single instance of a zero-sized
    /// component with the given ID.
    fn new_for_single_zero_sized_instance(component_id: ComponentID) -> SingleInstance<Self> {
        Self::new_for_single_instance(component_id, 0, AlignedByteVec::new(Alignment::new(1)))
    }

    /// Initializes an empty storage with preallocated capacity for
    /// the given number of component instances of type `C`.
    pub fn with_capacity<C: Component>(component_count: usize) -> Self {
        let component_size = mem::size_of::<C>();
        Self::new(
            C::component_id(),
            0,
            component_size,
            AlignedByteVec::with_capacity(Alignment::of::<C>(), component_count * component_size),
        )
    }

    /// Copies the bytes in the given view into a new storage.
    pub fn from_view<'a>(slice: impl ComponentSlice<'a>) -> Self {
        let view = slice.persistent_view();

        ComponentStorage {
            component_id: view.component_id(),
            component_count: view.component_count(),
            component_size: view.component_size(),
            bytes: AlignedByteVec::copied_from_slice(view.component_align, view.bytes),
        }
    }

    /// Copies the bytes in the given view representing a single component
    /// instance into a new storage.
    pub fn from_single_instance_view<'a>(
        slice: impl ComponentSlice<'a> + ComponentInstance,
    ) -> SingleInstance<Self> {
        SingleInstance::new_unchecked(Self::from_view(slice))
    }

    /// Returns the size of the storage in bytes.
    pub fn size(&self) -> usize {
        self.bytes.len()
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
        // Make sure not to call `cast_slice` on an empty slice, as
        // an empty slice is not guaranteed to have the correct alignment
        if self.bytes.is_empty() {
            &[]
        } else {
            bytemuck::cast_slice(&self.bytes)
        }
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
        // Make sure not to call `cast_slice` on an empty slice, as
        // an empty slice is not guaranteed to have the correct alignment
        if self.bytes.is_empty() {
            &mut []
        } else {
            bytemuck::cast_slice_mut(&mut self.bytes)
        }
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

    /// Adds the bytes in the given component array to the end of the
    /// storage.
    ///
    /// # Panics
    /// If the component ID associated with the given array does not
    /// correspond to the type the storage was initialized with.
    pub fn push_array(&mut self, array: &impl ComponentArray) {
        let ComponentView {
            component_id,
            component_count,
            component_size: _,
            component_align: _,
            bytes,
        } = array.view();
        self.validate_component_id(component_id);
        self.bytes.extend_from_slice(bytes);
        self.component_count += component_count;
    }

    /// Removes the component at the given index and makes the last component
    /// take its place (unless the one to remove is the last one).
    ///
    /// # Note
    /// `idx` refers to the whole component, not its byte boundary.
    ///
    /// # Returns
    /// A [`SingleInstance<ComponentStorage>`] with only the removed component.
    ///
    /// # Panics
    /// If `idx` is outside the bounds of the storage.
    pub fn swap_remove(&mut self, idx: usize) -> SingleInstance<Self> {
        assert!(
            idx < self.component_count(),
            "Index for component out of bounds"
        );

        let removed_component_data = if self.component_size > 0 {
            let component_to_remove_start = idx.checked_mul(self.component_size).unwrap();
            let data_size = self.bytes.len();

            let removed_component_data = Self::new_for_single_instance(
                self.component_id,
                self.component_size,
                AlignedByteVec::copied_from_slice(
                    self.bytes.alignment(),
                    &self.bytes[component_to_remove_start
                        ..component_to_remove_start + self.component_size],
                ),
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
            Self::new_for_single_zero_sized_instance(self.component_id)
        };

        self.component_count = self.component_count.checked_sub(1).unwrap();

        removed_component_data
    }

    /// Removes all components from the storage.
    pub fn clear(&mut self) {
        self.bytes.truncate(0);
        self.component_count = 0;
    }

    fn validate_component<C: Component>(&self) {
        self.validate_component_id(C::component_id());
    }

    fn validate_component_id(&self, component_id: ComponentID) {
        assert!(
            component_id == self.component_id,
            "Tried to use component storage with invalid component type"
        );
    }
}

impl ComponentArray for ComponentStorage {
    fn component_id(&self) -> ComponentID {
        self.component_id
    }

    fn component_count(&self) -> usize {
        self.component_count
    }

    fn component_size(&self) -> usize {
        self.component_size
    }

    fn component_align(&self) -> Alignment {
        self.bytes.alignment()
    }

    fn view(&self) -> ComponentView<'_> {
        ComponentView::new(
            self.component_id,
            self.component_count,
            self.component_size,
            self.bytes.alignment(),
            self.bytes.as_slice(),
        )
    }

    fn into_storage(self) -> ComponentStorage {
        self
    }
}

impl SingleInstance<ComponentStorage> {
    /// Converts this single-instance storage into a storage containing copies
    /// of the instance component data for the given number of instances.
    ///
    /// # Panics
    /// If `n_instances` is zero.
    pub fn duplicate_instance(self, n_instances: usize) -> ComponentStorage {
        assert_ne!(
            n_instances, 0,
            "Tried to duplicate component storage data zero times"
        );

        let mut storage = self.into_inner();

        if n_instances == 1 {
            return storage;
        }

        if storage.component_size() != 0 {
            let mut duplicated_bytes = AlignedByteVec::with_capacity(
                storage.bytes.alignment(),
                n_instances * storage.component_size(),
            );
            for _ in 0..n_instances {
                duplicated_bytes.extend_from_slice(&storage.bytes);
            }
            storage.bytes = duplicated_bytes;
        }
        storage.component_count = n_instances;

        storage
    }
}

impl<'a> ComponentView<'a> {
    /// Creates a new view to the given bytes for a component
    /// with the given ID, count, size and alignment.
    fn new(
        component_id: ComponentID,
        component_count: usize,
        component_size: usize,
        component_align: Alignment,
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
            component_align,
            bytes,
        }
    }

    /// Creates a new view to the given bytes for a single instance
    /// of the component with the given ID, count and size.
    fn new_for_single_instance(
        component_id: ComponentID,
        component_size: usize,
        component_align: Alignment,
        bytes: &'a [u8],
    ) -> SingleInstance<Self> {
        SingleInstance::new_unchecked(Self::new(
            component_id,
            1,
            component_size,
            component_align,
            bytes,
        ))
    }

    fn validate_component<C: Component>(&self) {
        self.validate_component_id(C::component_id());
    }

    fn validate_component_id(&self, component_id: ComponentID) {
        assert!(
            component_id == self.component_id,
            "Tried to use component byte view with invalid component type"
        );
    }
}

impl ComponentArray for ComponentView<'_> {
    fn component_id(&self) -> ComponentID {
        self.component_id
    }

    fn component_count(&self) -> usize {
        self.component_count
    }

    fn component_size(&self) -> usize {
        self.component_size
    }

    fn component_align(&self) -> Alignment {
        self.component_align
    }

    fn view(&self) -> ComponentView<'_> {
        self.clone()
    }

    fn into_storage(self) -> ComponentStorage {
        ComponentStorage::from_view(self)
    }
}

impl<'a> ComponentSlice<'a> for ComponentView<'a> {
    fn component_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    fn component_instances<C: Component>(&self) -> &'a [C] {
        self.validate_component::<C>();
        assert_ne!(
            mem::size_of::<C>(),
            0,
            "Tried to obtain slice of zero-sized component values from component view"
        );
        // Make sure not to call `cast_slice` on an empty slice, as
        // an empty slice is not guaranteed to have the correct alignment
        if self.bytes.is_empty() {
            &[]
        } else {
            bytemuck::cast_slice(self.bytes)
        }
    }
}

impl<C: Component> ComponentArray for &[C] {
    fn component_id(&self) -> ComponentID {
        C::component_id()
    }

    fn component_count(&self) -> usize {
        self.len()
    }

    fn component_size(&self) -> usize {
        mem::size_of::<C>()
    }

    fn component_align(&self) -> Alignment {
        Alignment::of::<C>()
    }

    fn view(&self) -> ComponentView<'_> {
        self.persistent_view()
    }

    fn into_storage(self) -> ComponentStorage {
        ComponentStorage::from_view(self)
    }
}

impl<'a, C: Component> ComponentSlice<'a> for &'a [C] {
    fn component_bytes(&self) -> &'a [u8] {
        if self.component_size() == 0 || self.component_count() == 0 {
            &[]
        } else {
            bytemuck::cast_slice(self)
        }
    }

    fn component_instances<C2: Component>(&self) -> &'a [C2] {
        self.persistent_view().component_instances()
    }
}

impl<const N: usize, C: Component> ComponentArray for &[C; N] {
    fn component_id(&self) -> ComponentID {
        C::component_id()
    }

    fn component_count(&self) -> usize {
        self.len()
    }

    fn component_size(&self) -> usize {
        mem::size_of::<C>()
    }

    fn component_align(&self) -> Alignment {
        Alignment::of::<C>()
    }

    fn view(&self) -> ComponentView<'_> {
        self.persistent_view()
    }

    fn into_storage(self) -> ComponentStorage {
        ComponentStorage::from_view(self)
    }
}

impl<'a, const N: usize, C: Component> ComponentSlice<'a> for &'a [C; N] {
    fn component_bytes(&self) -> &'a [u8] {
        if self.component_size() == 0 || self.component_count() == 0 {
            &[]
        } else {
            #[allow(clippy::explicit_auto_deref)]
            bytemuck::cast_slice(*self)
        }
    }

    fn component_instances<C2: Component>(&self) -> &'a [C2] {
        self.persistent_view().component_instances()
    }
}

impl<T> SingleInstance<T>
where
    T: CanHaveSingleInstance,
{
    /// Wraps the given container in a [`SingleInstance`].
    ///
    /// # Panics
    /// If the data in the container does not represent a single instance.
    pub fn new(container: T) -> Self {
        assert_eq!(container.instance_count(), 1);
        Self::new_unchecked(container)
    }

    /// Wraps the given container in a [`SingleInstance`].
    ///
    /// # Warning
    /// Does not verify that the data in the container represents a single
    /// instance.
    pub(crate) fn new_unchecked(container: T) -> Self {
        Self { container }
    }

    /// Unwraps the [`SingleInstance`] wrapped container.
    pub fn into_inner(self) -> T {
        self.container
    }
}

impl<T> Deref for SingleInstance<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl<T> ComponentArray for SingleInstance<T>
where
    T: ComponentArray,
{
    fn component_id(&self) -> ComponentID {
        T::component_id(&self.container)
    }

    fn component_count(&self) -> usize {
        T::component_count(&self.container)
    }

    fn component_size(&self) -> usize {
        T::component_size(&self.container)
    }

    fn component_align(&self) -> Alignment {
        T::component_align(&self.container)
    }

    fn view(&self) -> ComponentView<'_> {
        T::view(&self.container)
    }

    fn into_storage(self) -> ComponentStorage {
        self.into_inner().into_storage()
    }
}

impl<T> ComponentInstance for SingleInstance<T>
where
    T: ComponentArray,
{
    fn single_instance_view(&self) -> SingleInstance<ComponentView<'_>> {
        SingleInstance::new_unchecked(self.view())
    }
}

impl<T> CanHaveSingleInstance for T
where
    T: ComponentArray,
{
    fn instance_count(&self) -> usize {
        self.component_count()
    }
}

#[cfg(test)]
mod tests {
    use super::{super::Component, *};
    use bytemuck::Zeroable;

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Marked;

    #[repr(transparent)]
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
    fn creating_view_to_single_component_works() {
        let component = Byte(42);
        let reference = &component;
        let view = reference.view();
        assert_eq!(view.component_count(), 1);
        assert_eq!(view.component_size(), mem::size_of::<Byte>());
        assert_eq!(view.bytes.len(), 1);
        assert_eq!(view.bytes[0], 42);
        assert_eq!(view.component_instances::<Byte>()[0], component);
    }

    #[test]
    fn creating_persistent_view_to_single_component_works() {
        let component = Byte(42);
        let view = (&component).persistent_view();
        assert_eq!(view.component_count(), 1);
        assert_eq!(view.component_size(), mem::size_of::<Byte>());
        assert_eq!(view.bytes.len(), 1);
        assert_eq!(view.bytes[0], 42);
        assert_eq!(view.component_instances::<Byte>()[0], component);
    }

    #[test]
    fn creating_view_to_single_zero_sized_component_works() {
        let reference = &Marked;
        let view = reference.view();
        assert_eq!(view.component_count(), 1);
        assert_eq!(view.component_size(), 0);
        assert_eq!(view.bytes.len(), 0);
    }

    #[test]
    fn creating_persistent_view_to_single_zero_sized_component_works() {
        let view = (&Marked).persistent_view();
        assert_eq!(view.component_count(), 1);
        assert_eq!(view.component_size(), 0);
        assert_eq!(view.bytes.len(), 0);
    }

    #[test]
    fn creating_view_to_array_of_component_works() {
        let components = &[Byte(42), Byte(24)];
        let view = components.view();
        assert_eq!(view.component_count(), 2);
        assert_eq!(view.component_size(), mem::size_of::<Byte>());
        assert_eq!(view.bytes.len(), 2);
        assert_eq!(view.bytes, &[42, 24]);
        assert_eq!(view.component_instances::<Byte>(), components);
    }

    #[test]
    fn creating_persistent_view_to_array_of_component_works() {
        let components = &[Byte(42), Byte(24)];
        let view = components.persistent_view();
        assert_eq!(view.component_count(), 2);
        assert_eq!(view.component_size(), mem::size_of::<Byte>());
        assert_eq!(view.bytes.len(), 2);
        assert_eq!(view.bytes, &[42, 24]);
        assert_eq!(view.component_instances::<Byte>(), components);
    }

    #[test]
    fn creating_view_to_array_of_zero_sized_component_works() {
        let components = &[Marked, Marked];
        let view = components.view();
        assert_eq!(view.component_count(), 2);
        assert_eq!(view.component_size(), 0);
        assert_eq!(view.bytes.len(), 0);
    }

    #[test]
    fn creating_persistent_view_to_array_of_zero_sized_component_works() {
        let components = &[Marked, Marked];
        let view = components.persistent_view();
        assert_eq!(view.component_count(), 2);
        assert_eq!(view.component_size(), 0);
        assert_eq!(view.bytes.len(), 0);
    }

    #[test]
    fn creating_view_to_slice_of_component_works() {
        let components = [Byte(42), Byte(24)].as_slice();
        let view = components.view();
        assert_eq!(view.component_count(), 2);
        assert_eq!(view.component_size(), mem::size_of::<Byte>());
        assert_eq!(view.bytes.len(), 2);
        assert_eq!(view.bytes, &[42, 24]);
        assert_eq!(view.component_instances::<Byte>(), components);
    }

    #[test]
    fn creating_persistent_view_to_slice_of_component_works() {
        let components = [Byte(42), Byte(24)].as_slice();
        let view = components.persistent_view();
        assert_eq!(view.component_count(), 2);
        assert_eq!(view.component_size(), mem::size_of::<Byte>());
        assert_eq!(view.bytes.len(), 2);
        assert_eq!(view.bytes, &[42, 24]);
        assert_eq!(view.component_instances::<Byte>(), components);
    }

    #[test]
    #[should_panic]
    fn requesting_instances_of_wrong_component_from_component_view_fails() {
        let component = Byte(42);
        let view = (&component).persistent_view();
        view.component_instances::<Rectangle>();
    }

    #[test]
    #[should_panic]
    fn requesting_instances_of_zero_size_component_from_component_view_fails() {
        let component = Marked;
        let view = (&component).persistent_view();
        view.component_instances::<Marked>();
    }

    #[test]
    fn creating_storage_from_view_works() {
        let storage = ComponentStorage::from_view(&RECT_1);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.size(), mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1]);
    }

    #[test]
    fn creating_empty_storage_works() {
        let storage = ComponentStorage::with_capacity::<Rectangle>(10);
        assert_eq!(storage.component_count(), 0);
        assert_eq!(storage.size(), 0);
        assert_eq!(storage.slice::<Rectangle>(), &[]);
    }

    #[test]
    #[should_panic]
    fn requesting_slice_of_wrong_component_from_storage_fails() {
        let storage = ComponentStorage::from_view(&RECT_1);
        storage.slice::<Byte>();
    }

    #[test]
    #[should_panic]
    fn requesting_mutable_slice_of_wrong_component_from_storage_fails() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.slice_mut::<Byte>();
    }

    #[test]
    #[should_panic]
    fn requesting_slice_of_zero_sized_component_from_storage_fails() {
        let storage = ComponentStorage::from_view(&Marked);
        storage.slice::<Marked>();
    }

    #[test]
    #[should_panic]
    fn requesting_mutable_slice_of_zero_sized_component_from_storage_fails() {
        let mut storage = ComponentStorage::from_view(&Marked);
        storage.slice_mut::<Marked>();
    }

    #[test]
    fn modifying_stored_component_works() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.slice_mut::<Rectangle>()[0] = RECT_2;
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2]);
    }

    #[test]
    #[should_panic]
    fn pushing_different_component_to_storage_fails() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.push(&Byte(42));
    }

    #[test]
    fn pushing_component_to_storage_works() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.push(&RECT_2);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.size(), 2 * mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_2]);
    }

    #[test]
    fn pushing_zero_sized_component_to_storage_works() {
        let mut storage = ComponentStorage::from_view(&Marked);
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.size(), 0);
    }

    #[test]
    #[should_panic]
    fn pushing_component_array_of_different_type_to_storage_fails() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.push_array(&&Byte(42));
    }

    #[test]
    fn pushing_component_array_to_storage_works() {
        let mut storage = ComponentStorage::from_view(&RECT_2);
        storage.push_array(&&[RECT_1, RECT_2]);
        assert_eq!(storage.component_count(), 3);
        assert_eq!(storage.size(), 3 * mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2, RECT_1, RECT_2]);
    }

    #[test]
    fn pushing_zero_sized_component_array_to_storage_works() {
        let mut storage = ComponentStorage::from_view(&Marked);
        storage.push_array(&&[Marked, Marked]);
        assert_eq!(storage.component_count(), 3);
        assert_eq!(storage.size(), 0);
    }

    #[test]
    #[should_panic]
    fn removing_component_from_storage_with_invalid_idx_fails() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.swap_remove(1);
    }

    #[test]
    fn swap_removing_component_from_storage_works() {
        let rect_1_storage = ComponentStorage::from_single_instance_view(&RECT_1);
        let rect_2_storage = ComponentStorage::from_single_instance_view(&RECT_2);

        let mut storage = ComponentStorage::from_view(&RECT_1);
        assert_eq!(storage.swap_remove(0), rect_1_storage);
        assert_eq!(storage.slice::<Rectangle>(), &[]);

        storage.push(&RECT_1);
        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove(0), rect_1_storage);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2]);
        assert_eq!(storage.swap_remove(0), rect_2_storage);
        assert_eq!(storage.slice::<Rectangle>(), &[]);

        storage.push(&RECT_1);
        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove(1), rect_2_storage);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1]);

        storage.push(&RECT_2);
        storage.push(&RECT_3);
        assert_eq!(storage.swap_remove(1), rect_2_storage);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_3]);

        storage.push(&RECT_2);
        assert_eq!(storage.swap_remove(0), rect_1_storage);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2, RECT_3]);
    }

    #[test]
    fn swap_removing_zero_sized_component_from_storage_works() {
        let marked_storage = ComponentStorage::from_single_instance_view(&Marked);

        let mut storage = ComponentStorage::from_view(&Marked);
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.swap_remove(0), marked_storage);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.swap_remove(0), marked_storage);
        assert_eq!(storage.component_count(), 0);

        storage.push(&Marked);
        storage.push(&Marked);
        storage.push(&Marked);
        assert_eq!(storage.component_count(), 3);
        assert_eq!(storage.swap_remove(1), marked_storage);
        assert_eq!(storage.component_count(), 2);
        assert_eq!(storage.swap_remove(1), marked_storage);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.swap_remove(0), marked_storage);
        assert_eq!(storage.component_count(), 0);
    }

    #[test]
    fn duplicating_single_instance_storage_works() {
        let single_instance_storage = ComponentStorage::from_single_instance_view(&RECT_1);

        let storage = single_instance_storage.duplicate_instance(3);

        assert_eq!(storage.component_count(), 3);
        assert_eq!(storage.size(), 3 * mem::size_of::<Rectangle>());
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_1, RECT_1, RECT_1]);
    }

    #[test]
    fn clearing_empty_storage_works() {
        let mut storage = ComponentStorage::with_capacity::<Rectangle>(10);
        storage.clear();
        assert_eq!(storage.component_count(), 0);
        assert_eq!(storage.size(), 0);
        assert_eq!(storage.slice::<Rectangle>(), &[]);
    }

    #[test]
    fn clearing_single_component_storage_works() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.clear();
        assert_eq!(storage.component_count(), 0);
        assert_eq!(storage.size(), 0);
        assert_eq!(storage.slice::<Rectangle>(), &[]);
    }

    #[test]
    fn clearing_multi_component_storage_works() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.push(&RECT_2);
        storage.clear();
        assert_eq!(storage.component_count(), 0);
        assert_eq!(storage.size(), 0);
        assert_eq!(storage.slice::<Rectangle>(), &[]);
    }

    #[test]
    fn reusing_cleared_storage_works() {
        let mut storage = ComponentStorage::from_view(&RECT_1);
        storage.clear();
        storage.push(&RECT_2);
        assert_eq!(storage.component_count(), 1);
        assert_eq!(storage.slice::<Rectangle>(), &[RECT_2]);
    }
}
