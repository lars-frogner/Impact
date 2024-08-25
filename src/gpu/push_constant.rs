//! Management of push constants.

use crate::{
    camera::buffer::CameraGPUBufferManager,
    gpu::rendering::{
        postprocessing::capturing::CapturingCamera, surface::RenderingSurface, RenderingSystem,
    },
    light::buffer::LightGPUBufferManager,
};
use bytemuck::Pod;

/// What the data in a push constant represent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PushConstantVariant {
    InverseWindowDimensions,
    PixelCount,
    LightIdx,
    CascadeIdx,
    Exposure,
    InverseExposure,
    FrameCounter,
    CameraRotationQuaternion,
}

/// Specification for a push constant that can be passed to the GPU.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PushConstant {
    /// The meaning of the push constant data.
    variant: PushConstantVariant,
    /// The shader stages where the push constant will be accessible.
    stages: wgpu::ShaderStages,
}

/// Specification for a collection of push constants that can be passed to the
/// GPU together.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PushConstantGroup {
    push_constants: Vec<PushConstant>,
}

/// A specific stage a push constant in a [`PushConstantGroup`] can be accessed
/// from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PushConstantGroupStage {
    Vertex,
    Fragment,
    Compute,
}

impl PushConstantVariant {
    /// Returns the size in bytes of the push constant of this variant.
    pub const fn size(&self) -> u32 {
        match self {
            Self::InverseWindowDimensions => {
                RenderingSurface::inverse_window_dimensions_push_constant_size()
            }
            Self::PixelCount => RenderingSurface::pixel_count_push_constant_size(),
            Self::LightIdx => LightGPUBufferManager::light_idx_push_constant_size(),
            Self::CascadeIdx => LightGPUBufferManager::cascade_idx_push_constant_size(),
            Self::Exposure => CapturingCamera::exposure_push_constant_size(),
            Self::InverseExposure => CapturingCamera::inverse_exposure_push_constant_size(),
            Self::FrameCounter => RenderingSystem::frame_counter_push_constant_size(),
            Self::CameraRotationQuaternion => {
                CameraGPUBufferManager::camera_rotation_quaternion_push_constant_size()
            }
        }
    }
}

impl PushConstant {
    /// Defines a new push constant with the given variant and stages.
    ///
    /// # Panics
    /// If the set of stages is invalid for push constants.
    pub fn new(variant: PushConstantVariant, stages: wgpu::ShaderStages) -> Self {
        assert!(
            stages == wgpu::ShaderStages::VERTEX
                || stages == wgpu::ShaderStages::FRAGMENT
                || stages == wgpu::ShaderStages::VERTEX_FRAGMENT
                || stages == wgpu::ShaderStages::COMPUTE,
            "invalid shader stages for push constant: {:?}",
            stages
        );
        Self { variant, stages }
    }

    /// Defines a new push constant with the given variant visible in the vertex
    /// shader stage.
    pub fn new_for_vertex(variant: PushConstantVariant) -> Self {
        Self::new(variant, wgpu::ShaderStages::VERTEX)
    }

    /// Defines a new push constant with the given variant visible in the
    /// fragment shader stage.
    pub fn new_for_fragment(variant: PushConstantVariant) -> Self {
        Self::new(variant, wgpu::ShaderStages::FRAGMENT)
    }

    /// Defines a new push constant with the given variant visible in the vertex
    /// and fragment shader stages.
    pub fn new_for_vertex_fragment(variant: PushConstantVariant) -> Self {
        Self::new(variant, wgpu::ShaderStages::VERTEX_FRAGMENT)
    }

    /// Defines a new push constant with the given variant visible in a compute
    /// shader.
    pub fn new_for_compute(variant: PushConstantVariant) -> Self {
        Self::new(variant, wgpu::ShaderStages::COMPUTE)
    }

    /// Returns the meaning of the push constant data.
    pub const fn variant(&self) -> PushConstantVariant {
        self.variant
    }

    /// Returns the shader stages where the push constant will be accessible.
    pub const fn stages(&self) -> wgpu::ShaderStages {
        self.stages
    }
}

impl PushConstantGroup {
    /// Creates a new empty push constant group.
    pub const fn new() -> Self {
        Self {
            push_constants: Vec::new(),
        }
    }

    /// Creates a push constant group for the given variants visible in the
    /// vertex shader stage.
    ///
    /// # Note
    /// The order of the variants must match the order in the shader. Also be
    /// careful with alignment: implicit padding requirements between fields
    /// in the push constant struct in the shader may cause the fields to be
    /// mapped to unexpected push constant ranges. In a double-push constant
    /// struct, this can be avoided by putting the larger push constant first.
    pub fn for_vertex(variants: impl IntoIterator<Item = PushConstantVariant>) -> Self {
        variants
            .into_iter()
            .map(PushConstant::new_for_vertex)
            .collect()
    }

    /// Creates a push constant group for the given variants visible in the
    /// fragment shader stage.
    ///
    /// # Note
    /// The order of the variants must match the order in the shader. Also be
    /// careful with alignment: implicit padding requirements between fields
    /// in the push constant struct in the shader may cause the fields to be
    /// mapped to unexpected push constant ranges. In a double-push constant
    /// struct, this can be avoided by putting the larger push constant first.
    pub fn for_fragment(variants: impl IntoIterator<Item = PushConstantVariant>) -> Self {
        variants
            .into_iter()
            .map(PushConstant::new_for_fragment)
            .collect()
    }

    /// Creates a push constant group for the given variants visible in the
    /// vertex and fragment shader stages.
    ///
    /// # Note
    /// The order of the variants must match the order in the shader. Also be
    /// careful with alignment: implicit padding requirements between fields
    /// in the push constant struct in the shader may cause the fields to be
    /// mapped to unexpected push constant ranges. In a double-push constant
    /// struct, this can be avoided by putting the larger push constant first.
    pub fn for_vertex_fragment(variants: impl IntoIterator<Item = PushConstantVariant>) -> Self {
        variants
            .into_iter()
            .map(PushConstant::new_for_vertex_fragment)
            .collect()
    }

    /// Creates a push constant group for the given variants visible in a
    /// compute shader.
    ///
    /// # Note
    /// The order of the variants must match the order in the shader. Also be
    /// careful with alignment: implicit padding requirements between fields
    /// in the push constant struct in the shader may cause the fields to be
    /// mapped to unexpected push constant ranges. In a double-push constant
    /// struct, this can be avoided by putting the larger push constant first.
    pub fn for_compute(variants: impl IntoIterator<Item = PushConstantVariant>) -> Self {
        variants
            .into_iter()
            .map(PushConstant::new_for_compute)
            .collect()
    }

    /// Returns all push constants present the group.
    pub fn push_constants(&self) -> &[PushConstant] {
        &self.push_constants
    }

    /// Returns the index of the push constant of the given variant within the
    /// subset of push constants in the group that are accessible from the given
    /// stage, or [`None`] if unavailable.
    pub fn find_idx_for_stage(
        &self,
        variant: PushConstantVariant,
        stage: PushConstantGroupStage,
    ) -> Option<usize> {
        self.find_idx_for_stages(variant, stage.into())
    }

    /// Returns an iterator over each push constant in the group that is
    /// accessible from the given stage.
    pub fn iter_for_stage(
        &self,
        stage: PushConstantGroupStage,
    ) -> impl Iterator<Item = &PushConstant> {
        let stages = stage.into();
        self.push_constants
            .iter()
            .filter(move |push_constant| push_constant.stages().contains(stages))
    }

    /// Returns the [`wgpu::PushConstantRange`]s for the group, for use in
    /// creating a pipeline layout.
    pub fn create_ranges(&self) -> Vec<wgpu::PushConstantRange> {
        if self.push_constants.is_empty() {
            return Vec::new();
        }

        let mut current_stages = self.push_constants[0].stages();
        let mut ranges = Vec::with_capacity(2);
        let mut range_start = 0;
        let mut range_end = 0;

        for push_constant in &self.push_constants {
            if push_constant.stages() != current_stages {
                ranges.push(wgpu::PushConstantRange {
                    stages: current_stages,
                    range: range_start..range_end,
                });
                range_start = range_end;
            }
            current_stages = push_constant.stages();
            range_end += push_constant.variant().size();
        }

        ranges.push(wgpu::PushConstantRange {
            stages: current_stages,
            range: range_start..range_end,
        });

        ranges
    }

    /// Makes the appropriate call to [`wgpu::RenderPass::set_push_constants`]
    /// with the provided value for the push constant with the given variant if
    /// that push constant is present in the group.
    pub fn set_push_constant_for_render_pass_if_present<T: Pod>(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        variant: PushConstantVariant,
        get_push_constant_value: impl FnOnce() -> T,
    ) {
        self.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                render_pass.set_push_constants(stages, offset, data);
            },
            variant,
            get_push_constant_value,
        );
    }

    /// Makes the appropriate call to [`wgpu::ComputePass::set_push_constants`]
    /// with the provided value for the push constant with the given variant if
    /// that push constant is present in the group.
    pub fn set_push_constant_for_compute_pass_if_present<T: Pod>(
        &self,
        compute_pass: &mut wgpu::ComputePass<'_>,
        variant: PushConstantVariant,
        get_push_constant_value: impl FnOnce() -> T,
    ) {
        self.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                assert_eq!(stages, wgpu::ShaderStages::COMPUTE);
                compute_pass.set_push_constants(offset, data);
            },
            variant,
            get_push_constant_value,
        );
    }

    /// Adds the given push constant to the group.
    ///
    /// # Panics
    /// If a push constant with the same variant (regardless of stages) is
    /// already present.
    pub fn add_push_constant(&mut self, push_constant: PushConstant) {
        assert!(!self.has_variant(push_constant.variant()));

        let idx = self
            .push_constants
            .iter()
            .position(|existing_push_constant| {
                shader_stages_order(push_constant.stages())
                    < shader_stages_order(existing_push_constant.stages())
            })
            .unwrap_or(self.push_constants.len());

        self.push_constants.insert(idx, push_constant);
    }

    fn has_variant(&self, variant: PushConstantVariant) -> bool {
        self.push_constants
            .iter()
            .any(|push_constant| push_constant.variant() == variant)
    }

    fn find_idx_for_stages(
        &self,
        variant: PushConstantVariant,
        stages: wgpu::ShaderStages,
    ) -> Option<usize> {
        let mut idx = 0;
        for push_constant in &self.push_constants {
            if push_constant.stages().contains(stages) {
                if push_constant.variant() == variant {
                    return Some(idx);
                }
                idx += 1;
            }
        }
        None
    }

    fn set_push_constant_for_pass_if_present<T: Pod>(
        &self,
        set_push_constant: impl FnOnce(wgpu::ShaderStages, u32, &[u8]),
        variant: PushConstantVariant,
        get_push_constant_value: impl FnOnce() -> T,
    ) {
        let mut offset = 0;
        let mut stages = wgpu::ShaderStages::empty();

        for push_constant in &self.push_constants {
            if push_constant.variant() == variant {
                stages = push_constant.stages();
                break;
            }
            offset += push_constant.variant().size();
        }

        if stages == wgpu::ShaderStages::empty() {
            return;
        }

        let value = get_push_constant_value();
        let data = bytemuck::bytes_of(&value);
        assert_eq!(data.len(), variant.size() as usize);

        set_push_constant(stages, offset, data);
    }
}

impl Default for PushConstantGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl From<PushConstant> for PushConstantGroup {
    fn from(push_constant: PushConstant) -> Self {
        let mut group = Self::new();
        group.add_push_constant(push_constant);
        group
    }
}

impl FromIterator<PushConstant> for PushConstantGroup {
    fn from_iter<T: IntoIterator<Item = PushConstant>>(iter: T) -> Self {
        let mut group = Self::new();
        for push_constant in iter {
            group.add_push_constant(push_constant);
        }
        group
    }
}

impl From<PushConstantGroupStage> for wgpu::ShaderStages {
    fn from(stage: PushConstantGroupStage) -> Self {
        match stage {
            PushConstantGroupStage::Vertex => wgpu::ShaderStages::VERTEX,
            PushConstantGroupStage::Fragment => wgpu::ShaderStages::FRAGMENT,
            PushConstantGroupStage::Compute => wgpu::ShaderStages::COMPUTE,
        }
    }
}

fn shader_stages_order(stages: wgpu::ShaderStages) -> u8 {
    match stages {
        wgpu::ShaderStages::VERTEX => 0,
        wgpu::ShaderStages::VERTEX_FRAGMENT => 1,
        wgpu::ShaderStages::FRAGMENT => 2,
        wgpu::ShaderStages::COMPUTE => 3,
        _ => panic!("unsupported shader stages: {:?}", stages),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use PushConstantVariant::*;

    #[test]
    #[should_panic]
    fn creating_push_constant_with_invalid_stages_fails() {
        PushConstant::new(LightIdx, wgpu::ShaderStages::empty());
    }

    #[test]
    fn adding_single_vertex_push_constant_to_group_works() {
        let mut group = PushConstantGroup::new();
        let push_constant = PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX);
        group.add_push_constant(push_constant.clone());
        assert_eq!(group.push_constants(), &[push_constant]);
    }

    #[test]
    fn adding_two_vertex_push_constants_to_group_works() {
        let mut group = PushConstantGroup::new();
        let push_constant_1 = PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX);
        let push_constant_2 = PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX);
        group.add_push_constant(push_constant_1.clone());
        group.add_push_constant(push_constant_2.clone());
        assert_eq!(group.push_constants(), &[push_constant_1, push_constant_2]);
    }

    #[test]
    fn adding_fragment_then_vertex_push_constant_to_group_gives_correct_order() {
        let mut group = PushConstantGroup::new();
        let push_constant_1 = PushConstant::new(LightIdx, wgpu::ShaderStages::FRAGMENT);
        let push_constant_2 = PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX);
        group.add_push_constant(push_constant_1.clone());
        group.add_push_constant(push_constant_2.clone());
        assert_eq!(group.push_constants(), &[push_constant_2, push_constant_1]);
    }

    #[test]
    fn adding_fragment_then_vertex_then_fragment_push_constant_to_group_gives_correct_order() {
        let mut group = PushConstantGroup::new();
        let push_constant_1 = PushConstant::new(LightIdx, wgpu::ShaderStages::FRAGMENT);
        let push_constant_2 = PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX);
        let push_constant_3 =
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::FRAGMENT);
        group.add_push_constant(push_constant_1.clone());
        group.add_push_constant(push_constant_2.clone());
        group.add_push_constant(push_constant_3.clone());
        assert_eq!(
            group.push_constants(),
            &[push_constant_2, push_constant_1, push_constant_3]
        );
    }

    #[test]
    fn adding_push_constants_with_each_stages_to_group_gives_correct_order() {
        let mut group = PushConstantGroup::new();
        let push_constant_1 =
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::VERTEX_FRAGMENT);
        let push_constant_2 = PushConstant::new(LightIdx, wgpu::ShaderStages::COMPUTE);
        let push_constant_3 = PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX);
        let push_constant_4 = PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT);
        group.add_push_constant(push_constant_1.clone());
        group.add_push_constant(push_constant_2.clone());
        group.add_push_constant(push_constant_3.clone());
        group.add_push_constant(push_constant_4.clone());
        assert_eq!(
            group.push_constants(),
            &[
                push_constant_3,
                push_constant_1,
                push_constant_4,
                push_constant_2
            ]
        );
    }

    #[test]
    fn collecting_to_group_gives_correct_order() {
        let push_constant_1 =
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::VERTEX_FRAGMENT);
        let push_constant_2 = PushConstant::new(LightIdx, wgpu::ShaderStages::COMPUTE);
        let push_constant_3 = PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX);
        let push_constant_4 = PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT);

        let group: PushConstantGroup = [
            push_constant_1.clone(),
            push_constant_2.clone(),
            push_constant_3.clone(),
            push_constant_4.clone(),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            group.push_constants(),
            &[
                push_constant_3,
                push_constant_1,
                push_constant_4,
                push_constant_2
            ]
        );
    }

    #[test]
    #[should_panic]
    fn adding_same_push_constant_variant_to_group_twice_fails() {
        let mut group = PushConstantGroup::new();
        group.add_push_constant(PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX));
        group.add_push_constant(PushConstant::new(LightIdx, wgpu::ShaderStages::FRAGMENT));
    }

    #[test]
    fn finding_index_for_stage_in_empty_group_gives_none() {
        let group = PushConstantGroup::new();
        assert!(group
            .find_idx_for_stage(LightIdx, PushConstantGroupStage::Vertex)
            .is_none());
    }

    #[test]
    fn finding_index_in_group_for_missing_stage_gives_none() {
        let group: PushConstantGroup =
            PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX).into();
        assert!(group
            .find_idx_for_stage(LightIdx, PushConstantGroupStage::Fragment)
            .is_none());
    }

    #[test]
    fn finding_index_in_group_for_missing_variant_gives_none() {
        let group: PushConstantGroup =
            PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX).into();
        assert!(group
            .find_idx_for_stage(CascadeIdx, PushConstantGroupStage::Vertex)
            .is_none());
    }

    #[test]
    fn finding_index_in_single_element_group_works() {
        let group: PushConstantGroup =
            PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX).into();
        assert_eq!(
            group.find_idx_for_stage(LightIdx, PushConstantGroupStage::Vertex),
            Some(0)
        );
    }

    #[test]
    fn finding_index_in_two_vertex_element_group_works() {
        let group: PushConstantGroup = [
            PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX),
            PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            group.find_idx_for_stage(LightIdx, PushConstantGroupStage::Vertex),
            Some(0)
        );
        assert_eq!(
            group.find_idx_for_stage(CascadeIdx, PushConstantGroupStage::Vertex),
            Some(1)
        );
    }

    #[test]
    fn finding_index_in_one_vertex_and_one_vertex_fragment_element_group_works() {
        let group: PushConstantGroup = [
            PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX_FRAGMENT),
            PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            group.find_idx_for_stage(LightIdx, PushConstantGroupStage::Vertex),
            Some(1)
        );
        assert_eq!(
            group.find_idx_for_stage(LightIdx, PushConstantGroupStage::Fragment),
            Some(0)
        );
        assert_eq!(
            group.find_idx_for_stage(CascadeIdx, PushConstantGroupStage::Vertex),
            Some(0)
        );
    }

    #[test]
    fn finding_index_in_group_with_each_stages_works() {
        let group: PushConstantGroup = [
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::VERTEX_FRAGMENT),
            PushConstant::new(LightIdx, wgpu::ShaderStages::COMPUTE),
            PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX),
            PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            group.find_idx_for_stage(InverseWindowDimensions, PushConstantGroupStage::Vertex),
            Some(1)
        );
        assert_eq!(
            group.find_idx_for_stage(InverseWindowDimensions, PushConstantGroupStage::Fragment),
            Some(0)
        );
        assert_eq!(
            group.find_idx_for_stage(LightIdx, PushConstantGroupStage::Compute),
            Some(0)
        );
        assert_eq!(
            group.find_idx_for_stage(CascadeIdx, PushConstantGroupStage::Vertex),
            Some(0)
        );
        assert_eq!(
            group.find_idx_for_stage(Exposure, PushConstantGroupStage::Fragment),
            Some(1)
        );
    }

    #[test]
    fn iterating_for_stage_in_group_with_each_stages_works() {
        let push_constant_1 =
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::VERTEX_FRAGMENT);
        let push_constant_2 = PushConstant::new(LightIdx, wgpu::ShaderStages::COMPUTE);
        let push_constant_3 = PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX);
        let push_constant_4 = PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT);

        let group: PushConstantGroup = [
            push_constant_1.clone(),
            push_constant_2.clone(),
            push_constant_3.clone(),
            push_constant_4.clone(),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            group
                .iter_for_stage(PushConstantGroupStage::Vertex)
                .collect::<Vec<_>>(),
            vec![&push_constant_3, &push_constant_1]
        );
        assert_eq!(
            group
                .iter_for_stage(PushConstantGroupStage::Fragment)
                .collect::<Vec<_>>(),
            vec![&push_constant_1, &push_constant_4]
        );
        assert_eq!(
            group
                .iter_for_stage(PushConstantGroupStage::Compute)
                .collect::<Vec<_>>(),
            vec![&push_constant_2]
        );
    }

    #[test]
    fn creating_ranges_for_empty_group_gives_empty_vec() {
        let group = PushConstantGroup::new();
        assert!(group.create_ranges().is_empty());
    }

    #[test]
    fn creating_ranges_for_single_element_group_works() {
        let group: PushConstantGroup =
            PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX).into();

        let ranges = group.create_ranges();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].range, 0..LightIdx.size());
        assert_eq!(ranges[0].stages, wgpu::ShaderStages::VERTEX);
    }

    #[test]
    fn creating_ranges_for_two_fragment_element_group_works() {
        let group: PushConstantGroup = [
            PushConstant::new(LightIdx, wgpu::ShaderStages::FRAGMENT),
            PushConstant::new(CascadeIdx, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        let ranges = group.create_ranges();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].range, 0..(LightIdx.size() + CascadeIdx.size()));
        assert_eq!(ranges[0].stages, wgpu::ShaderStages::FRAGMENT);
    }

    #[test]
    fn creating_ranges_for_one_vertex_and_one_fragment_element_group_works() {
        let group: PushConstantGroup = [
            PushConstant::new(FrameCounter, wgpu::ShaderStages::VERTEX),
            PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        let ranges = group.create_ranges();

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].range, 0..FrameCounter.size());
        assert_eq!(ranges[0].stages, wgpu::ShaderStages::VERTEX);
        assert_eq!(
            ranges[1].range,
            FrameCounter.size()..(FrameCounter.size() + Exposure.size())
        );
        assert_eq!(ranges[1].stages, wgpu::ShaderStages::FRAGMENT);
    }

    #[test]
    fn creating_ranges_for_group_with_each_stages_works() {
        let group: PushConstantGroup = [
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::VERTEX_FRAGMENT),
            PushConstant::new(LightIdx, wgpu::ShaderStages::COMPUTE),
            PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX),
            PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        let ranges = group.create_ranges();

        assert_eq!(ranges.len(), 4);

        let mut offset = 0;
        assert_eq!(ranges[0].range, offset..offset + CascadeIdx.size());
        assert_eq!(ranges[0].stages, wgpu::ShaderStages::VERTEX);
        offset += CascadeIdx.size();

        assert_eq!(
            ranges[1].range,
            offset..offset + InverseWindowDimensions.size()
        );
        assert_eq!(ranges[1].stages, wgpu::ShaderStages::VERTEX_FRAGMENT);
        offset += InverseWindowDimensions.size();

        assert_eq!(ranges[2].range, offset..offset + Exposure.size());
        assert_eq!(ranges[2].stages, wgpu::ShaderStages::FRAGMENT);
        offset += Exposure.size();

        assert_eq!(ranges[3].range, offset..offset + LightIdx.size());
        assert_eq!(ranges[3].stages, wgpu::ShaderStages::COMPUTE);
    }

    #[test]
    fn creating_ranges_for_one_vertex_and_three_fragment_group_works() {
        let group: PushConstantGroup = [
            PushConstant::new(FrameCounter, wgpu::ShaderStages::VERTEX),
            PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT),
            PushConstant::new(LightIdx, wgpu::ShaderStages::FRAGMENT),
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        let ranges = group.create_ranges();

        assert_eq!(ranges.len(), 2);

        let mut offset = 0;
        assert_eq!(ranges[0].range, offset..offset + FrameCounter.size());
        assert_eq!(ranges[0].stages, wgpu::ShaderStages::VERTEX);
        offset += FrameCounter.size();

        assert_eq!(
            ranges[1].range,
            offset..offset + Exposure.size() + LightIdx.size() + InverseWindowDimensions.size()
        );
        assert_eq!(ranges[1].stages, wgpu::ShaderStages::FRAGMENT);
    }

    #[test]
    fn setting_push_constant_for_pass_for_empty_group_does_nothing() {
        let group = PushConstantGroup::new();
        let mut called = false;
        group.set_push_constant_for_pass_if_present(
            |_, _, _| {
                called = true;
            },
            LightIdx,
            || 0_u32,
        );
        assert!(!called);
    }

    #[test]
    fn setting_push_constant_for_pass_for_missing_variant_does_nothing() {
        let group: PushConstantGroup =
            PushConstant::new(LightIdx, wgpu::ShaderStages::VERTEX).into();
        let mut called = false;
        group.set_push_constant_for_pass_if_present(
            |_, _, _| {
                called = true;
            },
            CascadeIdx,
            || 0_u32,
        );
        assert!(!called);
    }

    #[test]
    fn setting_push_constant_for_pass_for_group_with_each_stages_works() {
        let group: PushConstantGroup = [
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::VERTEX_FRAGMENT),
            PushConstant::new(LightIdx, wgpu::ShaderStages::COMPUTE),
            PushConstant::new(CascadeIdx, wgpu::ShaderStages::VERTEX),
            PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            CascadeIdx,
            || 1_u32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::VERTEX));
        assert_eq!(set_offset, Some(0));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&1_u32).to_vec()));

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            InverseWindowDimensions,
            || [1_u32, 2],
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::VERTEX_FRAGMENT));
        assert_eq!(set_offset, Some(CascadeIdx.size()));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&[1_u32, 2]).to_vec()));

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            Exposure,
            || 5.0_f32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::FRAGMENT));
        assert_eq!(
            set_offset,
            Some(CascadeIdx.size() + InverseWindowDimensions.size())
        );
        assert_eq!(set_data, Some(bytemuck::bytes_of(&5.0_f32).to_vec()));

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            LightIdx,
            || 3_u32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::COMPUTE));
        assert_eq!(
            set_offset,
            Some(CascadeIdx.size() + InverseWindowDimensions.size() + Exposure.size())
        );
        assert_eq!(set_data, Some(bytemuck::bytes_of(&3_u32).to_vec()));
    }

    #[test]
    fn setting_push_constant_for_pass_for_one_vertex_and_three_fragment_group_works() {
        let group: PushConstantGroup = [
            PushConstant::new(FrameCounter, wgpu::ShaderStages::VERTEX),
            PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT),
            PushConstant::new(LightIdx, wgpu::ShaderStages::FRAGMENT),
            PushConstant::new(InverseWindowDimensions, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            FrameCounter,
            || 1_u32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::VERTEX));
        assert_eq!(set_offset, Some(0));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&1_u32).to_vec()));

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            Exposure,
            || 2.0_f32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::FRAGMENT));
        assert_eq!(set_offset, Some(FrameCounter.size()));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&2.0_f32).to_vec()));

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            LightIdx,
            || 2_u32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::FRAGMENT));
        assert_eq!(set_offset, Some(FrameCounter.size() + Exposure.size()));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&2_u32).to_vec()));

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            InverseWindowDimensions,
            || [1_u32, 2],
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::FRAGMENT));
        assert_eq!(
            set_offset,
            Some(FrameCounter.size() + Exposure.size() + LightIdx.size())
        );
        assert_eq!(set_data, Some(bytemuck::bytes_of(&[1_u32, 2]).to_vec()));
    }

    #[test]
    fn setting_push_constant_for_pass_for_group_with_each_stages_works_3() {
        let group: PushConstantGroup = [
            PushConstant::new(FrameCounter, wgpu::ShaderStages::VERTEX),
            PushConstant::new(Exposure, wgpu::ShaderStages::FRAGMENT),
        ]
        .into_iter()
        .collect();

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            FrameCounter,
            || 1_u32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::VERTEX));
        assert_eq!(set_offset, Some(0));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&1_u32).to_vec()));

        let mut set_stages = None;
        let mut set_offset = None;
        let mut set_data = None;
        group.set_push_constant_for_pass_if_present(
            |stages, offset, data| {
                set_stages = Some(stages);
                set_offset = Some(offset);
                set_data = Some(data.to_vec());
            },
            Exposure,
            || 2.0_f32,
        );
        assert_eq!(set_stages, Some(wgpu::ShaderStages::FRAGMENT));
        assert_eq!(set_offset, Some(FrameCounter.size()));
        assert_eq!(set_data, Some(bytemuck::bytes_of(&2.0_f32).to_vec()));
    }
}
