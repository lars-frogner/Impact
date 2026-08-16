//! Management of immediate data.

use bytemuck::Pod;

pub const MAX_TOTAL_IMMEDIATE_SIZE: usize = 256;

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

/// Stager for immediate data that should be set for a pass. Individual pieces
/// of immediate data can be written to their appropriate locations in a staging
/// buffer, and the staging buffer can later be set as an immediate (from offset
/// zero) for the pass. This avoids redundant immediate setting commands.
#[derive(Debug)]
pub struct ImmediateStager<'a, I: Immediate> {
    group: &'a ImmediateGroup<I>,
    immediate_data: [u8; MAX_TOTAL_IMMEDIATE_SIZE],
}

pub trait ImmediateSettablePass {
    fn set_staged_immediates(&mut self, offset: u32, data: &[u8]);
}

impl ImmediateSettablePass for wgpu::RenderPass<'_> {
    fn set_staged_immediates(&mut self, offset: u32, data: &[u8]) {
        self.set_immediates(offset, data);
    }
}

impl ImmediateSettablePass for wgpu::ComputePass<'_> {
    fn set_staged_immediates(&mut self, offset: u32, data: &[u8]) {
        self.set_immediates(offset, data);
    }
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

    /// Creates an [`ImmediateStager`] that can be used to stage and set
    /// immediate data for the given pass.
    pub fn create_data_stager<'a>(&'a self) -> ImmediateStager<'a, I> {
        ImmediateStager {
            group: self,
            immediate_data: [0; _],
        }
    }

    /// Adds the given immediate to the group.
    ///
    /// # Panics
    /// - If the same immediate is already present.
    /// - If the total size of all immediates exceeds
    ///   [`MAX_TOTAL_IMMEDIATE_SIZE`].
    pub fn add_immediate(&mut self, immediate: I) {
        assert!(!self.immediates.contains(&immediate));
        self.immediates.push(immediate);
        assert!(self.total_size() as usize <= MAX_TOTAL_IMMEDIATE_SIZE);
    }

    fn set_immediate_data_if_present<T: Pod>(
        &self,
        immediate_data: &mut [u8; MAX_TOTAL_IMMEDIATE_SIZE],
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
            offset += existing_immediate.size() as usize;
        }

        if !found {
            return;
        }

        let value = get_immediate_value();
        let data = bytemuck::bytes_of(&value);
        assert_eq!(data.len(), immediate.size() as usize);

        let end = offset + data.len();
        assert!(end <= MAX_TOTAL_IMMEDIATE_SIZE);

        immediate_data[offset..end].copy_from_slice(data);
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

impl<'a, I: Immediate> ImmediateStager<'a, I> {
    /// Stages the provided value for the given immediate if that immediate is
    /// present in the group. Any previously staged value for the immediate will
    /// be overwritten.
    ///
    /// # Panics
    /// If the size of the data does not match the registered size of the
    /// immediate.
    pub fn stage_immediate_data_if_present<T: Pod>(
        &mut self,
        immediate: I,
        get_immediate_data: impl FnOnce() -> T,
    ) {
        self.group.set_immediate_data_if_present(
            &mut self.immediate_data,
            immediate,
            get_immediate_data,
        );
    }

    /// Sets the last staged value of each immediate for the given pass.
    /// Immediates that have not been staged will be set to zero. Calling this
    /// method will not reset the staged values.
    pub fn set_staged_immediate_data(&mut self, pass: &mut impl ImmediateSettablePass) {
        pass.set_staged_immediates(0, &self.immediate_data[..self.group.total_size() as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TestImmediate::*;
    use impact_containers::{AlignedByteVec, Alignment};
    use std::mem;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestImmediate {
        Const4A,
        Const4B,
        Const4C,
        Const4D,
        Const8A,
    }

    impl Immediate for TestImmediate {
        fn size(&self) -> u32 {
            match self {
                Self::Const4A | Self::Const4B | Self::Const4C | Self::Const4D => 4,
                Self::Const8A => 8,
            }
        }
    }

    #[derive(Clone, Debug)]
    struct MockPass {
        data: AlignedByteVec,
    }

    impl MockPass {
        fn new() -> Self {
            Self {
                data: AlignedByteVec::new(Alignment::EIGHT),
            }
        }

        fn n_bytes(&self) -> u32 {
            self.data.len() as u32
        }

        fn get_value<T: Pod>(&self, offset: u32) -> Option<T> {
            let offset = offset as usize;
            let end = offset + mem::size_of::<T>();
            if end > self.data.len() {
                return None;
            }
            Some(*bytemuck::from_bytes(&self.data[offset..end]))
        }
    }

    impl ImmediateSettablePass for MockPass {
        fn set_staged_immediates(&mut self, offset: u32, data: &[u8]) {
            assert_eq!(offset, 0);
            self.data.resize(data.len(), 0);
            self.data.copy_from_slice(data);
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
        group.add_immediate(Const4A);
        assert_eq!(group.immediates(), &[Const4A]);
        assert_eq!(group.total_size(), Const4A.size());
    }

    #[test]
    fn converting_single_immediate_to_group_works() {
        let group: ImmediateGroup<_> = Const4A.into();
        assert_eq!(group.immediates(), &[Const4A]);
    }

    #[test]
    fn adding_multiple_immediates_to_group_preserves_order() {
        let mut group = ImmediateGroup::new();
        group.add_immediate(Const8A);
        group.add_immediate(Const4C);
        group.add_immediate(Const4A);
        assert_eq!(group.immediates(), &[Const8A, Const4C, Const4A]);
    }

    #[test]
    fn collecting_to_group_preserves_order() {
        let group: ImmediateGroup<_> = [Const8A, Const4A, Const4B, Const4C].into_iter().collect();
        assert_eq!(group.immediates(), &[Const8A, Const4A, Const4B, Const4C]);
    }

    #[test]
    #[should_panic]
    fn adding_same_immediate_to_group_twice_fails() {
        let mut group = ImmediateGroup::new();
        group.add_immediate(Const4A);
        group.add_immediate(Const4A);
    }

    #[test]
    fn total_size_of_group_is_sum_of_immediate_sizes() {
        let group: ImmediateGroup<_> = [Const8A, Const4D, Const4B].into_iter().collect();
        assert_eq!(
            group.total_size(),
            Const8A.size() + Const4D.size() + Const4B.size()
        );
    }

    #[test]
    fn setting_immediate_for_empty_group_does_nothing() {
        let group = ImmediateGroup::new();
        let mut pass = MockPass::new();

        let mut stager = group.create_data_stager();
        stager.stage_immediate_data_if_present(Const4A, || 0_u32);
        stager.set_staged_immediate_data(&mut pass);

        assert_eq!(pass.n_bytes(), 0);
    }

    #[test]
    fn setting_missing_immediate_does_nothing() {
        let group: ImmediateGroup<_> = Const4A.into();
        let mut pass = MockPass::new();

        let mut stager = group.create_data_stager();
        stager.stage_immediate_data_if_present(Const4B, || 1_u32);
        stager.set_staged_immediate_data(&mut pass);

        assert_eq!(pass.n_bytes(), 4);
        assert_eq!(pass.get_value(0), Some(0_u32));
    }

    #[test]
    fn setting_first_immediate_in_group_uses_zero_offset() {
        let group: ImmediateGroup<_> = [Const8A, Const4B, Const4C].into_iter().collect();
        let mut pass = MockPass::new();

        let mut stager = group.create_data_stager();
        stager.stage_immediate_data_if_present(Const8A, || 1_u64);
        stager.set_staged_immediate_data(&mut pass);

        assert_eq!(
            pass.n_bytes(),
            Const8A.size() + Const4B.size() + Const4C.size()
        );
        assert_eq!(pass.get_value(0), Some(1_u64));
    }

    #[test]
    fn setting_immediates_in_group_uses_offsets_of_preceding_immediates() {
        let group: ImmediateGroup<_> = [Const8A, Const4B, Const4C, Const4A].into_iter().collect();
        let mut pass = MockPass::new();

        let mut stager = group.create_data_stager();
        stager.stage_immediate_data_if_present(Const4C, || 5_f32);
        stager.stage_immediate_data_if_present(Const4B, || 1_u32);
        stager.stage_immediate_data_if_present(Const4A, || 3_u32);
        stager.set_staged_immediate_data(&mut pass);

        assert_eq!(
            pass.n_bytes(),
            Const8A.size() + Const4B.size() + Const4C.size() + Const4A.size()
        );
        assert_eq!(pass.get_value(Const8A.size()), Some(1_u32));
        assert_eq!(pass.get_value(Const8A.size() + Const4B.size()), Some(5_f32));
        assert_eq!(
            pass.get_value(Const8A.size() + Const4B.size() + Const4C.size()),
            Some(3_u32)
        );
    }

    #[test]
    fn setting_immediates_multiple_times_does_not_reset_staging_buffer() {
        let group: ImmediateGroup<_> = [Const4A, Const4B, Const4C].into_iter().collect();
        let mut pass = MockPass::new();

        let mut stager = group.create_data_stager();

        stager.stage_immediate_data_if_present(Const4B, || 1_u32);
        stager.set_staged_immediate_data(&mut pass);

        assert_eq!(
            pass.n_bytes(),
            Const4A.size() + Const4B.size() + Const4C.size()
        );
        assert_eq!(pass.get_value(0), Some(0_u32));
        assert_eq!(pass.get_value(Const4A.size()), Some(1_u32));
        assert_eq!(pass.get_value(Const4A.size() + Const4B.size()), Some(0_u32));

        stager.stage_immediate_data_if_present(Const4A, || 2_u32);
        stager.stage_immediate_data_if_present(Const4C, || 3_u32);
        stager.set_staged_immediate_data(&mut pass);

        assert_eq!(
            pass.n_bytes(),
            Const4A.size() + Const4B.size() + Const4C.size()
        );
        assert_eq!(pass.get_value(0), Some(2_u32));
        assert_eq!(pass.get_value(Const4A.size()), Some(1_u32));
        assert_eq!(pass.get_value(Const4A.size() + Const4B.size()), Some(3_u32));
    }

    #[test]
    #[should_panic]
    fn setting_immediate_with_value_of_wrong_size_panics() {
        let group: ImmediateGroup<_> = Const8A.into();
        let mut pass = MockPass::new();
        let mut stager = group.create_data_stager();
        stager.stage_immediate_data_if_present(Const8A, || 0_u32);
        stager.set_staged_immediate_data(&mut pass);
    }
}
