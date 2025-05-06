//! Commands for operating the rendering system.

use super::RenderingSystem;
use crate::{
    engine::command::{ModifiedActiveState, ToActiveState},
    gpu::{
        rendering::{
            RenderCommandManager,
            postprocessing::{
                capturing::tone_mapping::ToneMappingMethod,
                command::{
                    PostprocessingCommand, ToExposure, ToRenderAttachmentQuantity,
                    ToToneMappingMethod,
                },
            },
        },
        texture::attachment::RenderAttachmentQuantity,
    },
};
use anyhow::Result;
use roc_codegen::roc;

#[roc(prefix = "Engine")]
#[derive(Clone, Debug)]
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
        to.set(&mut self.config.shadow_mapping_enabled)
    }

    pub fn set_wireframe_mode(&mut self, to: ToActiveState) -> ModifiedActiveState {
        let state = to.set(&mut self.config.wireframe_mode_on);
        if state.changed {
            *self.render_command_manager.write().unwrap() = RenderCommandManager::new(
                &self.graphics_device,
                &mut self.shader_manager.write().unwrap(),
                &mut self.render_attachment_texture_manager.write().unwrap(),
                &self.config,
            );
        }
        state
    }

    pub fn set_render_pass_timings(&mut self, to: ToActiveState) -> ModifiedActiveState {
        let state = to.set(&mut self.config.timings_enabled);
        self.timestamp_query_manager
            .set_enabled(self.config.timings_enabled);
        state
    }
}
