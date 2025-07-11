//! Commands for operating the rendering system.

use super::RenderingSystem;
use crate::{
    command::{ActiveState, ModifiedActiveState, ToActiveState},
    gpu::rendering::{
        RenderCommandManager,
        postprocessing::{
            self,
            command::{
                PostprocessingCommand, ToExposure, ToRenderAttachmentQuantity, ToToneMappingMethod,
            },
        },
    },
};
use anyhow::Result;
use impact_gpu::wgpu;
use impact_rendering::{
    attachment::RenderAttachmentQuantity,
    postprocessing::capturing::dynamic_range_compression::ToneMappingMethod,
};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderingCommand {
    Postprocessing(PostprocessingCommand),
    SetShadowMapping(ToActiveState),
    SetWireframeMode(ToActiveState),
    SetRenderPassTimings(ToActiveState),
}

impl RenderingSystem {
    pub fn set_ambient_occlusion(&self, to: ToActiveState) -> ModifiedActiveState {
        postprocessing::command::set_ambient_occlusion(&mut self.postprocessor.write().unwrap(), to)
    }

    pub fn set_temporal_anti_aliasing(&self, to: ToActiveState) -> ModifiedActiveState {
        postprocessing::command::set_temporal_anti_aliasing(
            &mut self.postprocessor.write().unwrap(),
            to,
        )
    }

    pub fn set_bloom(&self, to: ToActiveState) -> ModifiedActiveState {
        postprocessing::command::set_bloom(&mut self.postprocessor.write().unwrap(), to)
    }

    pub fn set_tone_mapping_method(&self, to: ToToneMappingMethod) -> ToneMappingMethod {
        postprocessing::command::set_tone_mapping_method(
            &mut self.postprocessor.write().unwrap(),
            to,
        )
    }

    pub fn set_exposure(&self, to: ToExposure) {
        postprocessing::command::set_exposure(&mut self.postprocessor.write().unwrap(), to);
    }

    pub fn set_render_attachment_visualization(&self, to: ToActiveState) -> ModifiedActiveState {
        postprocessing::command::set_render_attachment_visualization(
            &mut self.postprocessor.write().unwrap(),
            to,
        )
    }

    pub fn set_visualized_render_attachment_quantity(
        &self,
        to: ToRenderAttachmentQuantity,
    ) -> Result<RenderAttachmentQuantity> {
        postprocessing::command::set_visualized_render_attachment_quantity(
            &mut self.postprocessor.write().unwrap(),
            to,
        )
    }

    pub fn set_shadow_mapping(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(self.shadow_mapping_enabled_mut())
    }

    pub fn set_wireframe_mode(&mut self, to: ToActiveState) -> ModifiedActiveState {
        if !self
            .graphics_device()
            .supports_features(wgpu::Features::POLYGON_MODE_LINE)
            && to != ToActiveState::Disabled
        {
            impact_log::warn!(
                "Not enabling wireframe mode due to missing graphics device features"
            );
            return ModifiedActiveState {
                state: ActiveState::Disabled,
                changed: false,
            };
        }

        let state = to.set(&mut self.basic_config.wireframe_mode_on);
        if state.changed {
            *self.render_command_manager.write().unwrap() = RenderCommandManager::new(
                &self.graphics_device,
                &self.rendering_surface,
                &mut self.shader_manager.write().unwrap(),
                &mut self.render_attachment_texture_manager.write().unwrap(),
                &self.bind_group_layout_registry,
                &self.basic_config,
            );
        }
        state
    }

    pub fn set_render_pass_timings(&mut self, to: ToActiveState) -> ModifiedActiveState {
        if !self
            .graphics_device()
            .supports_features(wgpu::Features::TIMESTAMP_QUERY)
            && to != ToActiveState::Disabled
        {
            impact_log::warn!(
                "Not enabling timestamp queries due to missing graphics device features"
            );
            return ModifiedActiveState {
                state: ActiveState::Disabled,
                changed: false,
            };
        }

        let state = to.set(&mut self.basic_config.timings_enabled);
        self.timestamp_query_manager
            .set_enabled(self.basic_config.timings_enabled);
        state
    }
}
