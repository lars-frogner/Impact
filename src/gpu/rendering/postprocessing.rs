//! Application of postprocessing.

pub mod ambient_occlusion;
pub mod capturing;
pub mod gaussian_blur;

use crate::{
    gpu::{
        compute::GPUComputationLibrary,
        push_constant::{PushConstant, PushConstantVariant},
        rendering::render_command::{
            OutputAttachmentSampling, RenderCommandSpecification, RenderCommandState,
            RenderPassHints, RenderPassSpecification,
        },
        shader::{MaterialShaderInput, PassthroughShaderInput, ShaderManager},
        storage::StorageGPUBufferManager,
        texture::attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
        GraphicsDevice,
    },
    material::{MaterialID, MaterialLibrary, MaterialSpecification},
    mesh::{VertexAttributeSet, SCREEN_FILLING_QUAD_MESH_ID},
};
use ambient_occlusion::AmbientOcclusionConfig;
use capturing::{CapturingCamera, CapturingCameraConfig};
use impact_utils::hash64;

/// Manager of materials and render commands for postprocessing effects.
#[derive(Clone, Debug)]
pub struct Postprocessor {
    ambient_occlusion_enabled: bool,
    ambient_occlusion_commands: Vec<RenderCommandSpecification>,
    capturing_camera: CapturingCamera,
}

impl Postprocessor {
    /// Creates a new postprocessor along with the associated materials,
    /// computations and render commands according to the given configuration.
    pub fn new(
        graphics_device: &GraphicsDevice,
        material_library: &mut MaterialLibrary,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
        gpu_computation_library: &mut GPUComputationLibrary,
        ambient_occlusion_config: &AmbientOcclusionConfig,
        capturing_camera_settings: &CapturingCameraConfig,
    ) -> Self {
        let ambient_occlusion_commands =
            ambient_occlusion::setup_ambient_occlusion_materials_and_render_commands(
                graphics_device,
                material_library,
                ambient_occlusion_config,
            );

        let capturing_camera = CapturingCamera::new(
            graphics_device,
            material_library,
            shader_manager,
            render_attachment_texture_manager,
            storage_gpu_buffer_manager,
            gpu_computation_library,
            capturing_camera_settings,
        );

        Self {
            ambient_occlusion_enabled: ambient_occlusion_config.initially_enabled,
            ambient_occlusion_commands,
            capturing_camera,
        }
    }

    /// Returns an iterator over the specifications for all postprocessing
    /// render commands, in the order in which they are to be performed.
    pub fn render_commands(&self) -> impl Iterator<Item = RenderCommandSpecification> + '_ {
        assert_eq!(self.ambient_occlusion_commands.len(), 3);
        self.ambient_occlusion_commands
            .iter()
            .cloned()
            .chain(self.capturing_camera.render_commands())
    }

    /// Returns an iterator over the current states of all postprocessing render
    /// commands, in the same order as from [`Self::render_commands`].
    pub fn render_command_states(&self) -> impl Iterator<Item = RenderCommandState> + '_ {
        assert_eq!(self.ambient_occlusion_commands.len(), 3);
        [
            !self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
        ]
        .into_iter()
        .map(RenderCommandState::active_if)
        .chain(self.capturing_camera.render_command_states())
    }

    /// Returns a reference to the capturing camera.
    pub fn capturing_camera(&self) -> &CapturingCamera {
        &self.capturing_camera
    }

    /// Returns a mutable reference to the capturing camera.
    pub fn capturing_camera_mut(&mut self) -> &mut CapturingCamera {
        &mut self.capturing_camera
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&mut self) {
        self.ambient_occlusion_enabled = !self.ambient_occlusion_enabled;
    }
}

fn setup_passthrough_material_and_render_pass(
    material_library: &mut MaterialLibrary,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> RenderCommandSpecification {
    let (material_id, material_specification) = setup_passthrough_material(
        material_library,
        input_render_attachment_quantity,
        output_render_attachment_quantity,
    );
    define_passthrough_pass(
        material_id,
        material_specification.render_pass_hints(),
        input_render_attachment_quantity,
        output_render_attachment_quantity,
    )
}

fn setup_passthrough_material(
    material_library: &mut MaterialLibrary,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> (MaterialID, &MaterialSpecification) {
    let material_id = MaterialID(hash64!(format!(
        "PassthroughMaterial{{ input: {}, output: {} }}",
        input_render_attachment_quantity, output_render_attachment_quantity,
    )));
    (
        material_id,
        material_library
            .material_specification_entry(material_id)
            .or_insert_with(|| {
                create_passthrough_material(
                    input_render_attachment_quantity,
                    output_render_attachment_quantity,
                )
            }),
    )
}

fn create_passthrough_material(
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> MaterialSpecification {
    MaterialSpecification::new(
        VertexAttributeSet::POSITION,
        VertexAttributeSet::empty(),
        input_render_attachment_quantity.flag(),
        output_render_attachment_quantity.flag(),
        None,
        Vec::new(),
        RenderPassHints::NO_DEPTH_PREPASS.union(RenderPassHints::NO_CAMERA),
        MaterialShaderInput::Passthrough(PassthroughShaderInput {
            input_texture_and_sampler_bindings: input_render_attachment_quantity.bindings(),
        }),
    )
}

fn define_passthrough_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> RenderCommandSpecification {
    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        input_render_attachment_quantities: input_render_attachment_quantity.flag(),
        output_render_attachment_quantities: output_render_attachment_quantity.flag(),
        output_attachment_sampling: OutputAttachmentSampling::Single,
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints,
        label: format!(
            "Passthrough pass: {} -> {}",
            input_render_attachment_quantity, output_render_attachment_quantity
        ),
        ..Default::default()
    })
}
