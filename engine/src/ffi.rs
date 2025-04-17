//! Utilities for FFI.

use anyhow::{Result, anyhow};
use impact_containers::{AlignedByteVec, Alignment};
use impact_ecs::component::{ComponentID, ComponentStorage, SingleInstance};

/// The expected layout is a packed sequence of component structures of the
/// following form:
/// ```
/// {
///     component_id: u64,
///     component_size: u64,
///     alignment: u64,
///     component_bytes: [u8; component_size],
/// }
/// ```
pub fn deserialize_components_for_single_entity(
    component_bytes: &[u8],
) -> Result<Vec<SingleInstance<ComponentStorage>>> {
    let mut cursor = ByteCursor::new("create_entity", component_bytes);
    let mut components = Vec::with_capacity(16);

    while cursor.is_inside() {
        let component_id = cursor.take_u64().map(ComponentID::from_u64)?;
        let size = cursor.take_usize()?;
        let alignment = cursor.take_usize().and_then(Alignment::try_new)?;

        let bytes = cursor.take(size)?;
        let aligned_bytes = AlignedByteVec::copied_from_slice(alignment, bytes);
        let component = ComponentStorage::new_for_single_instance(component_id, aligned_bytes);
        components.push(component);
    }

    Ok(components)
}

/// The expected layout is a packed sequence of multi-instance component
/// structures of the following form:
/// ```
/// {
///     component_id: u64,
///     component_size: u64,
///     alignment: u64,
///     component_count: u64,
///     component_bytes: [u8; component_count * component_size],
/// }
/// ```
/// Returns the resulting list of entity IDs encoded as `u64`s.
pub fn deserialize_components_for_multiple_entities(
    component_bytes: &[u8],
) -> Result<Vec<ComponentStorage>> {
    let mut cursor = ByteCursor::new("create_entities", component_bytes);
    let mut components = Vec::with_capacity(16);

    while cursor.is_inside() {
        let component_id = cursor.take_u64().map(ComponentID::from_u64)?;
        let size = cursor.take_usize()?;
        let alignment = cursor.take_usize().and_then(Alignment::try_new)?;
        let count = cursor.take_usize()?;

        let size_of_all = count.checked_mul(size).unwrap();
        let bytes = cursor.take(size_of_all)?;
        let aligned_bytes = AlignedByteVec::copied_from_slice(alignment, bytes);
        let component = ComponentStorage::new(component_id, count, size, aligned_bytes);
        components.push(component);
    }

    Ok(components)
}

#[derive(Debug)]
struct ByteCursor<'a> {
    error_context: &'a str,
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(error_context: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            error_context,
            bytes,
            cursor: 0,
        }
    }

    fn is_inside(&self) -> bool {
        self.cursor < self.bytes.len()
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.cursor + n;
        let slice = self.bytes.get(self.cursor..end).ok_or_else(|| {
            anyhow!(
                "{}: out of bounds when slicing {}..{} of {}-byte input",
                self.error_context,
                self.cursor,
                end,
                self.bytes.len()
            )
        })?;
        self.cursor = end;
        Ok(slice)
    }

    fn take_u64(&mut self) -> Result<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes(bytes.try_into()?))
    }

    fn take_usize(&mut self) -> Result<usize> {
        Ok(usize::try_from(self.take_u64()?)?)
    }
}
