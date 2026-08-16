//! Management of immediate data.

use bytemuck::Pod;

/// Represents the meaning and size of an immediate value.
pub trait Immediate: Copy + PartialEq {
    /// Returns the size in bytes of the immediate.
    fn size(&self) -> u32;
}

/// Specification for a collection of immediates that can be passed to the GPU
/// together.
///
/// # Note
/// The order of the immediates must match the order in the shader. Also be
/// careful with alignment: implicit padding requirements between fields in the
/// immediate struct in the shader may cause the fields to be mapped to
/// unexpected offsets. In a two-field immediate struct, this can be avoided by
/// putting the larger immediate first.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImmediateGroup<I> {
    immediates: Vec<I>,
}

impl<I: Immediate> ImmediateGroup<I> {
    /// Creates a new empty immediate group.
    pub const fn new() -> Self {
        Self {
            immediates: Vec::new(),
        }
    }

    /// Returns all immediates present the group.
    pub fn immediates(&self) -> &[I] {
        &self.immediates
    }

    /// Returns the total size of all the immediates in the group.
    pub fn total_size(&self) -> u32 {
        self.immediates
            .iter()
            .map(|immediate| immediate.size())
            .sum()
    }

    /// Makes the appropriate call to [`wgpu::RenderPass::set_immediates`] with
    /// the provided value for the given immediate if that immediate is present
    /// in the group.
    pub fn set_immediate_for_render_pass_if_present<T: Pod>(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        variant: I,
        get_immediate_value: impl FnOnce() -> T,
    ) {
        self.set_immediate_if_present(
            |offset, data| {
                render_pass.set_immediates(offset, data);
            },
            variant,
            get_immediate_value,
        );
    }

    /// Makes the appropriate call to [`wgpu::ComputePass::set_immediates`] with
    /// the provided value for the given immediate if that immediate is present
    /// in the group.
    pub fn set_immediate_for_compute_pass_if_present<T: Pod>(
        &self,
        compute_pass: &mut wgpu::ComputePass<'_>,
        variant: I,
        get_immediate_value: impl FnOnce() -> T,
    ) {
        self.set_immediate_if_present(
            |offset, data| {
                compute_pass.set_immediates(offset, data);
            },
            variant,
            get_immediate_value,
        );
    }

    /// Adds the given immediate to the group.
    ///
    /// # Panics
    /// If the same immediate is already present.
    pub fn add_immediate(&mut self, immediate: I) {
        assert!(!self.immediates.contains(&immediate));
        self.immediates.push(immediate);
    }

    fn set_immediate_if_present<T: Pod>(
        &self,
        set_immediate: impl FnOnce(u32, &[u8]),
        immediate: I,
        get_immediate_value: impl FnOnce() -> T,
    ) {
        let mut offset = 0;
        let mut found = false;

        for existing_immediate in &self.immediates {
            if existing_immediate == &immediate {
                found = true;
                break;
            }
            offset += existing_immediate.size();
        }

        if !found {
            return;
        }

        let value = get_immediate_value();
        let data = bytemuck::bytes_of(&value);
        assert_eq!(data.len(), immediate.size() as usize);

        set_immediate(offset, data);
    }
}

impl<V: Immediate> Default for ImmediateGroup<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Immediate> From<V> for ImmediateGroup<V> {
    fn from(immediate: V) -> Self {
        let mut group = Self::new();
        group.add_immediate(immediate);
        group
    }
}

impl<V: Immediate> FromIterator<V> for ImmediateGroup<V> {
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        let mut group = Self::new();
        for immediate in iter {
            group.add_immediate(immediate);
        }
        group
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TestImmediate::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestImmediate {
        ConstA,
        ConstB,
        ConstC,
        ConstD,
        ConstE,
    }

    impl Immediate for TestImmediate {
        fn size(&self) -> u32 {
            match self {
                Self::ConstA | Self::ConstB | Self::ConstC | Self::ConstD => 4,
                Self::ConstE => 8,
            }
        }
    }

    #[test]
    fn new_group_is_empty() {
        let group = ImmediateGroup::<TestImmediate>::new();
        assert!(group.immediates().is_empty());
        assert_eq!(group.total_size(), 0);
    }

    #[test]
    fn default_group_is_empty() {
        let group = ImmediateGroup::<TestImmediate>::default();
        assert!(group.immediates().is_empty());
    }

    #[test]
    fn adding_single_immediate_to_group_works() {
        let mut group = ImmediateGroup::new();
        group.add_immediate(ConstA);
        assert_eq!(group.immediates(), &[ConstA]);
        assert_eq!(group.total_size(), ConstA.size());
    }

    #[test]
    fn converting_single_immediate_to_group_works() {
        let group: ImmediateGroup<_> = ConstA.into();
        assert_eq!(group.immediates(), &[ConstA]);
    }

    #[test]
    fn adding_multiple_immediates_to_group_preserves_order() {
        let mut group = ImmediateGroup::new();
        group.add_immediate(ConstC);
        group.add_immediate(ConstA);
        group.add_immediate(ConstE);
        assert_eq!(group.immediates(), &[ConstC, ConstA, ConstE]);
    }

    #[test]
    fn collecting_to_group_preserves_order() {
        let group: ImmediateGroup<_> = [ConstE, ConstA, ConstB, ConstC].into_iter().collect();
        assert_eq!(group.immediates(), &[ConstE, ConstA, ConstB, ConstC]);
    }

    #[test]
    #[should_panic]
    fn adding_same_immediate_to_group_twice_fails() {
        let mut group = ImmediateGroup::new();
        group.add_immediate(ConstA);
        group.add_immediate(ConstA);
    }

    #[test]
    fn total_size_of_group_is_sum_of_immediate_sizes() {
        let group: ImmediateGroup<_> = [ConstD, ConstE, ConstB].into_iter().collect();
        assert_eq!(
            group.total_size(),
            ConstD.size() + ConstE.size() + ConstB.size()
        );
    }

    #[test]
    fn setting_immediate_for_empty_group_does_nothing() {
        let group = ImmediateGroup::new();
        let mut called = false;
        group.set_immediate_if_present(
            |_, _| {
                called = true;
            },
            ConstA,
            || 0_u32,
        );
        assert!(!called);
    }

    #[test]
    fn setting_missing_immediate_does_nothing() {
        let group: ImmediateGroup<_> = ConstA.into();
        let mut called = false;
        group.set_immediate_if_present(
            |_, _| {
                called = true;
            },
            ConstB,
            || 0_u32,
        );
        assert!(!called);
    }

    #[test]
    fn setting_first_immediate_in_group_uses_zero_offset() {
        let group: ImmediateGroup<_> = [ConstB, ConstE, ConstC].into_iter().collect();

        let mut set_offset = None;
        let mut set_data = None;
        group.set_immediate_if_present(
            |offset, data| {
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            ConstB,
            || 1_u32,
        );
        assert_eq!(set_offset, Some(0));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&1_u32).to_vec()));
    }

    #[test]
    fn setting_immediates_in_group_uses_offsets_of_preceding_immediates() {
        let group: ImmediateGroup<_> = [ConstB, ConstE, ConstC, ConstA].into_iter().collect();

        let mut set_offset = None;
        let mut set_data = None;
        group.set_immediate_if_present(
            |offset, data| {
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            ConstE,
            || [1_u32, 2],
        );
        assert_eq!(set_offset, Some(ConstB.size()));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&[1_u32, 2]).to_vec()));

        let mut set_offset = None;
        let mut set_data = None;
        group.set_immediate_if_present(
            |offset, data| {
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            ConstC,
            || 5.0_f32,
        );
        assert_eq!(set_offset, Some(ConstB.size() + ConstE.size()));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&5.0_f32).to_vec()));

        let mut set_offset = None;
        let mut set_data = None;
        group.set_immediate_if_present(
            |offset, data| {
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            ConstA,
            || 3_u32,
        );
        assert_eq!(
            set_offset,
            Some(ConstB.size() + ConstE.size() + ConstC.size())
        );
        assert_eq!(set_data, Some(bytemuck::bytes_of(&3_u32).to_vec()));
    }

    #[test]
    #[should_panic]
    fn setting_immediate_with_value_of_wrong_size_fails() {
        let group: ImmediateGroup<_> = ConstA.into();
        group.set_immediate_if_present(|_, _| {}, ConstA, || [0_u32, 0]);
    }
}
