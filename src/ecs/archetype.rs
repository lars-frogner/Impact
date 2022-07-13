//! Organization of ECS entities into archetypes.

use super::{
    component::{Component, ComponentByteView, ComponentBytes, ComponentID, ComponentStorage},
    query::StorageAccess,
    util::KeyIndexMapper,
    world::{Entity, EntityID},
};
use anyhow::{anyhow, bail, Result};
use paste::paste;
use std::{
    any::TypeId,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    sync::RwLock,
};

/// Representation of an archetype.
///
/// An archetype refers to a specific set of [`Component`]s
/// that an entity can have. All entities with the exact
/// same set of components belong to the same archetype.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct Archetype {
    id: ArchetypeID,
}

/// Unique identifier for an [`Archetype`], obtained by hashing
/// the sorted list of component IDs defining the archetype.
pub type ArchetypeID = u64;

/// A table holding the component data belonging to all entities
/// with a specific archetype.
///
/// The component data is conceptually stored as a table in the
/// following manner:
/// ```txt
/// Entity ID |       Components         |
///           | Position  |   Velocity   |
/// -------------------------------------|
///      ID 0 | {x, y, z} | {vx, vy, vz} |
///      ID 1 | {x, y, z} | {vx, vy, vz} |
///      ID 2 | {x, y, z} | {vx, vy, vz} |
/// ```
/// Each column of component data exists in its own [`ComponentStorage`],
/// with components stored in the same order as the entities in
/// the first column.
///
/// Each `ComponentStorage` is protected from invalid concurrent
/// access by an individual [`RwLock`].
#[derive(Debug)]
pub struct ArchetypeTable {
    archetype: Archetype,
    /// A map from [`EntityID`] to the index of its components
    /// in the [`ComponentStorage`]s.
    entity_index_mapper: KeyIndexMapper<EntityID>,
    /// A map from [`ComponentID`] to the index of the corresponding
    /// [`ComponentStorage`] in the `component_storages` vector.
    component_index_map: HashMap<ComponentID, usize>,
    component_storages: Vec<RwLock<ComponentStorage>>,
}

/// Container holding the [`ComponentBytes`] for a set of
/// components making up a specific [`Archetype`].
#[derive(Clone, Debug)]
pub struct ArchetypeCompBytes {
    archetype: Archetype,
    component_bytes: Vec<ComponentBytes>,
}

/// Container holding the [`ComponentByteView`] for a set
/// of components making up a specific [`Archetype`].
///
/// Instances of this type can be constructed conveniently
/// by converting from a single reference or a tuple of
/// references to anything that implements [`Component`].
///
/// # Examples
/// ```
/// # use impact::ecs::{component::Component, archetype::ArchetypeCompByteView};
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
/// #
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod)]
/// # struct Position(f32, f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod)]
/// # struct Mass(f32);
/// #
/// // Create instances of two components
/// let position = Position(0.0, 0.0);
/// let mass = Mass(5.0);
///
/// // We can convert from a single component..
/// let mass_bytes: ArchetypeCompByteView = (&mass).into();
/// // .. or from a tuple of multiple components..
/// let pos_mass_bytes: ArchetypeCompByteView = (&position, &mass).try_into()?;
/// // .. or from an array if we use views to the raw bytes
/// let pos_mass_bytes: ArchetypeCompByteView = [
///     position.component_bytes(), mass.component_bytes()
/// ].try_into()?;
/// #
/// # Ok::<(), Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct ArchetypeCompByteView<'a> {
    archetype: Archetype,
    component_bytes: Vec<ComponentByteView<'a>>,
}

impl Archetype {
    /// Creates a new archetype defined by the component IDs
    /// in the given array. The order of the component IDs
    /// does not affect the result. The array is allowed
    /// to be empty.
    ///
    /// # Errors
    /// Returns an error if the same component ID occurs
    /// multiple times in the array.
    pub fn new_from_component_id_arr<const N: usize>(
        mut component_ids: [ComponentID; N],
    ) -> Result<Self> {
        component_ids.sort();
        Self::new_from_sorted_component_id_arr(component_ids)
    }

    /// Returns the unique ID defining this archetype.
    pub fn id(&self) -> ArchetypeID {
        self.id
    }

    fn new_from_sorted_component_id_arr<const N: usize>(
        component_ids: [ComponentID; N],
    ) -> Result<Self> {
        if N > 0 {
            // Verify that no component is represented multiple times
            let duplicates_exist =
                (1..N).any(|idx| component_ids[idx..].contains(&component_ids[idx - 1]));
            if duplicates_exist {
                bail!("Duplicate component ID when constructing archetype");
            }
        }

        Ok(Self::new_from_sorted_component_ids_unchecked(
            &component_ids,
        ))
    }

    fn new_from_sorted_component_ids_unchecked(component_ids: &[ComponentID]) -> Self {
        Self {
            id: Self::create_id_from_sorted_component_ids(component_ids),
        }
    }

    /// Obtains an archetype ID by hashing the slice of sorted component IDs.
    fn create_id_from_sorted_component_ids(component_ids: &[ComponentID]) -> ArchetypeID {
        let mut hasher = DefaultHasher::new();
        component_ids.hash(&mut hasher);
        hasher.finish()
    }
}

impl ArchetypeTable {
    /// Takes an [`Entity`] and references to all its component data
    /// (as an [`ArchetypeCompByteView`]), initializes a table for the
    /// corresponding [`Archetype`] and inserts the given data as
    /// the first row.
    pub fn new_with_entity(entity: Entity, components: ArchetypeCompByteView) -> Self {
        let ArchetypeCompByteView {
            archetype,
            component_bytes,
        } = components;
        Self {
            archetype,
            // Initialize mapper between entity ID and index in component storages
            entity_index_mapper: KeyIndexMapper::new_with_key(entity.id()),
            // For component IDs we don't need a full `KeyIndexMapper`, so we just
            // unwrap to the underlying `HashMap`
            component_index_map: KeyIndexMapper::new_with_keys(
                component_bytes.iter().map(ComponentByteView::component_id),
            )
            .into_map(),
            // Initialize storages with component data for the provided entity
            component_storages: component_bytes
                .into_iter()
                .map(|data| RwLock::new(ComponentStorage::new_with_bytes(data)))
                .collect(),
        }
    }

    /// Whether no entities remain in the table.
    pub fn is_empty(&self) -> bool {
        self.entity_index_mapper.is_empty()
    }

    /// Takes an [`Entity`] and references to all its component data
    /// (as an [`ArchetypeCompByteView`]) and appends the data as a row
    /// in the table.
    ///
    /// # Errors
    /// Returns an error if the archetype of `entity` or `components`
    /// differs from the archetype of the table.
    pub fn add_entity(&mut self, entity: Entity, components: ArchetypeCompByteView) -> Result<()> {
        let ArchetypeCompByteView {
            archetype,
            component_bytes,
        } = components;
        if entity.archetype_id() != self.archetype.id {
            bail!("Archetype of entity to add inconsistent with table archetype");
        }
        if archetype != self.archetype {
            bail!("Archetype of component data inconsistent with table archetype");
        }

        self.entity_index_mapper.push_key(entity.id());

        self.component_storages
            .iter_mut()
            .zip(component_bytes.into_iter())
            .for_each(|(storage, data)| storage.write().unwrap().push_bytes(data));
        Ok(())
    }

    /// Removes the given entity and all its data from the
    /// table.
    ///
    /// # Returns
    /// The removed component data.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The archetype of the entity differs from the archetype of the table.
    /// - The entity is not present in the table.
    pub fn remove_entity(&mut self, entity: &Entity) -> Result<ArchetypeCompBytes> {
        if entity.archetype_id() != self.archetype.id {
            bail!("Archetype of entity to remove inconsistent with table archetype");
        }
        if !self.entity_index_mapper.contains_key(entity.id()) {
            bail!("Entity to remove not present in archetype table");
        }

        // Remove the entity from the map and obtain the index
        // of the corresponing component data. We do a swap remove
        // in order to keep the index map consistent when we do a
        // swap remove of component data.
        let idx = self.entity_index_mapper.swap_remove_key(entity.id());

        // Perform an equivalent swap remove of the data at the index we found
        let removed_component_bytes = self
            .component_storages
            .iter_mut()
            .map(|storage| storage.write().unwrap().swap_remove_bytes(idx))
            .collect();

        Ok(ArchetypeCompBytes {
            archetype: self.archetype,
            component_bytes: removed_component_bytes,
        })
    }

    /// Provides access to a [`ComponentStorage`] (guarded by a [`RwLock`]).
    ///
    /// The component type to access is given by the `C` type parameter,
    /// while the `A` type parameter specifies what kind of access (i.e.
    /// read or write, see [`StorageAccess`]).
    ///
    /// # Panics
    /// If `C` is not one of the component types present in the table.
    pub fn access_component_storage<'w, 'g, C, A>(&'w self) -> A::Guard
    where
        C: Component,
        A: StorageAccess<'w, 'g, C>,
    {
        let component_id = C::component_id();
        let idx = self.component_index_map[&component_id];
        let storage = &self.component_storages[idx];
        A::access(storage)
    }
}

impl ArchetypeCompBytes {
    /// Returns the unique ID for the archetype corresponding
    /// to the set of components whose bytes are stored here.
    pub fn id(&self) -> ArchetypeID {
        self.archetype.id
    }

    /// Returns an [`ArchetypeCompByteView`] referencing the component
    /// bytes.
    pub fn as_ref(&self) -> ArchetypeCompByteView {
        ArchetypeCompByteView {
            archetype: self.archetype,
            component_bytes: self
                .component_bytes
                .iter()
                .map(ComponentBytes::as_ref)
                .collect(),
        }
    }
}

impl<'a> ArchetypeCompByteView<'a> {
    /// Returns the unique ID for the archetype corresponding
    /// to the set of components whose bytes are referenced here.
    pub fn id(&self) -> ArchetypeID {
        self.archetype.id
    }

    /// Includes the given component in the set of components
    /// whose bytes are referenced here. Note that this changes
    /// the corresponding archetype.
    ///
    /// # Errors
    /// Returns an error if the type of the given component is
    /// already present.
    pub fn add_component_bytes(&mut self, component_bytes: ComponentByteView<'a>) -> Result<()> {
        // Find where to insert the given component to keep
        // the components sorted by ID
        match self.component_bytes.binary_search_by_key(
            &component_bytes.component_id(),
            ComponentByteView::component_id,
        ) {
            // Panic if the component is already present, otherwise insert
            // at the correct position
            Ok(_) => {
                bail!("Component already exists for archetype")
            }
            Err(idx) => self.component_bytes.insert(idx, component_bytes),
        }

        // Update archetype
        self.archetype = Self::find_archetype_from_sorted_components(&self.component_bytes);

        Ok(())
    }

    /// Removes the component with the given ID from the set
    /// of components whose bytes are referenced here. Note
    /// that this changes the corresponding archetype.
    ///
    /// # Errors
    /// Returns an error if the component type to remove is
    /// not present.
    pub fn remove_component_with_id(&mut self, component_id: ComponentID) -> Result<()> {
        let idx = self
            .component_bytes
            .binary_search_by_key(&component_id, ComponentByteView::component_id)
            .map_err(|_| anyhow!("Tried to remove missing component"))?;

        self.component_bytes.remove(idx);

        // Update archetype
        self.archetype = Self::find_archetype_from_sorted_components(&self.component_bytes);

        Ok(())
    }

    fn find_archetype_from_sorted_components(
        component_data: &[ComponentByteView<'a>],
    ) -> Archetype {
        let component_ids: Vec<_> = component_data
            .iter()
            .map(ComponentByteView::component_id)
            .collect();
        Archetype::new_from_sorted_component_ids_unchecked(&component_ids)
    }
}

// Implement `TryFrom` so that an array of `ComponentByteView`s can
// be converted into an `ArchetypeCompByteView`.
impl<'a, const N: usize> TryFrom<[ComponentByteView<'a>; N]> for ArchetypeCompByteView<'a> {
    type Error = anyhow::Error;

    fn try_from(mut component_data: [ComponentByteView<'a>; N]) -> Result<Self> {
        // Make sure components are sorted by id
        component_data.sort_by_key(|data| data.component_id());

        // Use arbitrary type ID to initialize array for component IDs
        // (will be overwritten)
        let dummy_type_id = TypeId::of::<u8>();
        let mut component_ids = [dummy_type_id; N];

        // Populate array of component IDs
        component_ids
            .iter_mut()
            .zip(component_data.iter())
            .for_each(|(id, data)| *id = data.component_id());

        let archetype = Archetype::new_from_sorted_component_id_arr(component_ids)?;

        Ok(Self {
            archetype,
            component_bytes: component_data.to_vec(),
        })
    }
}

/// Macro for implementing [`From<C>`] or [`TryFrom<C>`] for
/// [`ArchetypeCompByteView`], where `C` respectively is a single
/// reference or tuple of references to [`Component`]s.
macro_rules! impl_archetype_conversion {
    ($c:ident) => {
        impl<'a, $c> From<&'a $c> for ArchetypeCompByteView<'a>
        where
            $c: 'a + Component,
        {
            fn from(component: &'a $c) -> Self {
                [component.component_bytes()].try_into().unwrap()
            }
        }
    };
    (($($c:ident),*)) => {
        impl<'a, $($c),*> TryFrom<($(&'a $c),*)> for ArchetypeCompByteView<'a>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(($(paste! { [<component_ $c>] }),*): ($(&'a $c),*)) -> Result<Self> {
                [$(paste! { [<component_ $c>] }.component_bytes()),*].try_into()
            }
        }
    };
}

impl_archetype_conversion!(C1);
impl_archetype_conversion!((C1, C2));
impl_archetype_conversion!((C1, C2, C3));
impl_archetype_conversion!((C1, C2, C3, C4));
impl_archetype_conversion!((C1, C2, C3, C4, C5));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8));

#[cfg(test)]
mod test {
    use super::*;
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    struct Byte(u8);

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    struct Position(f32, f32, f32);

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    struct Rectangle {
        center: [f32; 2],
        dimensions: [f32; 2],
    }

    const BYTE: Byte = Byte(42);
    const POS: Position = Position(-9.8, 12.5, 7.3);
    const RECT: Rectangle = Rectangle {
        center: [2.5, 2.0],
        dimensions: [12.3, 8.9],
    };

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_array_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = [BYTE.component_bytes(), BYTE.component_bytes()]
            .try_into()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_array_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = [
            BYTE.component_bytes(),
            POS.component_bytes(),
            BYTE.component_bytes(),
        ]
        .try_into()
        .unwrap();
    }

    #[test]
    fn valid_conversion_of_comp_arrays_to_byte_views_succeed() {
        let view: ArchetypeCompByteView = [].try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([]).unwrap()
        );

        let view: ArchetypeCompByteView = [BYTE.component_bytes()].try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id()]).unwrap()
        );

        let view: ArchetypeCompByteView = [BYTE.component_bytes(), POS.component_bytes()]
            .try_into()
            .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id(), Position::component_id()])
                .unwrap()
        );

        let view: ArchetypeCompByteView = [
            BYTE.component_bytes(),
            POS.component_bytes(),
            RECT.component_bytes(),
        ]
        .try_into()
        .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Byte::component_id(),
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );
    }

    #[test]
    fn order_of_comps_for_byte_view_does_not_matter() {
        let view_1: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        let view_2: ArchetypeCompByteView = (&POS, &BYTE, &RECT).try_into().unwrap();
        let view_3: ArchetypeCompByteView = (&RECT, &BYTE, &POS).try_into().unwrap();
        assert_eq!(view_2.archetype, view_1.archetype);
        assert_eq!(view_3.archetype, view_1.archetype);
    }

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_tuple_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = (&POS, &POS).try_into().unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_tuple_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = (&POS, &BYTE, &POS).try_into().unwrap();
    }

    #[test]
    fn valid_conversion_of_comp_tuples_to_byte_views_succeed() {
        let view: ArchetypeCompByteView = (&BYTE).into();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id()]).unwrap()
        );

        let view: ArchetypeCompByteView = (&BYTE, &POS).try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id(), Position::component_id()])
                .unwrap()
        );

        let view: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Byte::component_id(),
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );
    }

    #[test]
    fn adding_components_to_archetype_byte_view_works() {
        let mut view: ArchetypeCompByteView = [].try_into().unwrap();
        view.add_component_bytes(BYTE.component_bytes()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id()]).unwrap()
        );

        view.add_component_bytes(POS.component_bytes()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id(), Position::component_id()])
                .unwrap()
        );

        view.add_component_bytes(RECT.component_bytes()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Byte::component_id(),
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn adding_existing_component_to_archetype_byte_view_fails() {
        let mut view: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        view.add_component_bytes(POS.component_bytes()).unwrap();
    }

    #[test]
    fn removing_components_from_archetype_byte_view_works() {
        let mut view: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        view.remove_component_with_id(Byte::component_id()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );

        view.remove_component_with_id(Rectangle::component_id())
            .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Position::component_id()]).unwrap()
        );

        view.remove_component_with_id(Position::component_id())
            .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([]).unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn removing_missing_component_from_archetype_byte_view_fails() {
        let mut view: ArchetypeCompByteView = (&BYTE, &RECT).try_into().unwrap();
        view.remove_component_with_id(Position::component_id())
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn removing_component_from_empty_archetype_byte_view_fails() {
        let mut view: ArchetypeCompByteView = [].try_into().unwrap();
        view.remove_component_with_id(Position::component_id())
            .unwrap();
    }
}
