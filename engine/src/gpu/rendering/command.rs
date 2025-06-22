//! Commands for operating the rendering system.

use super::RenderingSystem;
use crate::{
    command::{ModifiedActiveState, ToActiveState},
    gpu::{
        rendering::attachment::RenderAttachmentQuantity,
        rendering::{
            RenderCommandManager,
            postprocessing::{
                capturing::dynamic_range_compression::ToneMappingMethod,
                command::{
                    PostprocessingCommand, ToExposure, ToRenderAttachmentQuantity,
                    ToToneMappingMethod,
                },
            },
        },
    },
};
use anyhow::Result;
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
        self.postprocessor
            .write()
            .unwrap()
            .set_ambient_occlusion(to)
    }

    pub fn set_temporal_anti_aliasing(&self, to: ToActiveState) -> ModifiedActiveState {
        self.postprocessor
            .write()
            .unwrap()
            .set_temporal_anti_aliasing(to)
    }

    pub fn set_bloom(&self, to: ToActiveState) -> ModifiedActiveState {
        self.postprocessor.write().unwrap().set_bloom(to)
    }

    pub fn set_tone_mapping_method(&self, to: ToToneMappingMethod) -> ToneMappingMethod {
        self.postprocessor
            .write()
            .unwrap()
            .set_tone_mapping_method(to)
    }

    pub fn set_exposure(&self, to: ToExposure) {
        self.postprocessor.write().unwrap().set_exposure(to);
    }

    pub fn set_render_attachment_visualization(&self, to: ToActiveState) -> ModifiedActiveState {
        self.postprocessor
            .write()
            .unwrap()
            .set_render_attachment_visualization(to)
    }

    pub fn set_visualized_render_attachment_quantity(
        &self,
        to: ToRenderAttachmentQuantity,
    ) -> Result<RenderAttachmentQuantity> {
        self.postprocessor
            .write()
            .unwrap()
            .set_visualized_render_attachment_quantity(to)
    }

    pub fn set_shadow_mapping(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(self.shadow_mapping_enabled_mut())
    }

    pub fn set_wireframe_mode(&mut self, to: ToActiveState) -> ModifiedActiveState {
        let state = to.set(&mut self.basic_config.wireframe_mode_on);
        if state.changed {
            *self.render_command_manager.write().unwrap() = RenderCommandManager::new(
                &self.graphics_device,
                &self.rendering_surface,
                &mut self.shader_manager.write().unwrap(),
                &mut self.render_attachment_texture_manager.write().unwrap(),
                &self.basic_config,
            );
        }
        state
    }

    pub fn set_render_pass_timings(&mut self, to: ToActiveState) -> ModifiedActiveState {
        let state = to.set(&mut self.basic_config.timings_enabled);
        self.timestamp_query_manager
            .set_enabled(self.basic_config.timings_enabled);
        state
    }
}
