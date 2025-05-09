//! Commands for operating the engine.

use super::Engine;
use crate::{
    control::{
        command::ControlCommand,
        motion::{MotionDirection, MotionState},
    },
    gpu::{
        rendering::{
            command::RenderingCommand,
            postprocessing::{
                capturing::tone_mapping::ToneMappingMethod,
                command::{
                    PostprocessingCommand, ToExposure, ToRenderAttachmentQuantity,
                    ToToneMappingMethod,
                },
            },
            screen_capture::command::{CaptureCommand, SaveShadowMapsFor},
        },
        texture::attachment::RenderAttachmentQuantity,
    },
    physics::{
        command::{PhysicsCommand, ToSimulationSpeedMultiplier, ToSubstepCount},
        fph,
    },
    scene::command::SceneCommand,
    skybox::Skybox,
    ui::command::{ToInteractionMode, UICommand},
};
use anyhow::Result;
use impact_ecs::world::Entity;
use roc_codegen::roc;

#[roc(prefix = "Engine")]
#[derive(Clone, Debug)]
pub enum EngineCommand {
    Rendering(RenderingCommand),
    Physics(PhysicsCommand),
    Scene(SceneCommand),
    Control(ControlCommand),
    UI(UICommand),
    Capture(CaptureCommand),
}

#[roc(prefix = "Engine")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToActiveState {
    Enabled,
    Disabled,
    Opposite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModifiedActiveState {
    pub state: ActiveState,
    pub changed: bool,
}

#[roc(prefix = "Engine")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveState {
    Enabled,
    Disabled,
}

impl Engine {
    pub fn execute_command(&self, command: EngineCommand) -> Result<()> {
        match command {
            EngineCommand::Rendering(command) => self.execute_rendering_command(command),
            EngineCommand::Physics(command) => self.execute_physics_command(command),
            EngineCommand::Scene(command) => self.execute_scene_command(command),
            EngineCommand::Control(command) => self.execute_control_command(command),
            EngineCommand::UI(command) => self.execute_ui_command(command),
            EngineCommand::Capture(command) => self.execute_capture_command(command),
        }
    }

    pub fn execute_rendering_command(&self, command: RenderingCommand) -> Result<()> {
        match command {
            RenderingCommand::Postprocessing(command) => {
                self.execute_rendering_postprocessing_command(command)?;
            }
            RenderingCommand::SetShadowMapping(to) => {
                self.set_shadow_mapping(to);
            }
            RenderingCommand::SetWireframeMode(to) => {
                self.set_wireframe_mode(to);
            }
            RenderingCommand::SetRenderPassTimings(to) => {
                self.set_render_pass_timings(to);
            }
        }
        Ok(())
    }

    pub fn execute_rendering_postprocessing_command(
        &self,
        command: PostprocessingCommand,
    ) -> Result<()> {
        match command {
            PostprocessingCommand::SetAmbientOcclusion(to) => {
                self.set_ambient_occlusion(to);
            }
            PostprocessingCommand::SetTemporalAntiAliasing(to) => {
                self.set_temporal_anti_aliasing(to);
            }
            PostprocessingCommand::SetBloom(to) => {
                self.set_bloom(to);
            }
            PostprocessingCommand::SetToneMappingMethod(to) => {
                self.set_tone_mapping_method(to);
            }
            PostprocessingCommand::SetExposure(to) => {
                self.set_exposure(to);
            }
            PostprocessingCommand::SetRenderAttachmentVisualization(to) => {
                self.set_render_attachment_visualization(to);
            }
            PostprocessingCommand::SetVisualizedRenderAttachmentQuantity(to) => {
                self.set_visualized_render_attachment_quantity(to)?;
            }
        }
        Ok(())
    }

    pub fn execute_physics_command(&self, command: PhysicsCommand) -> Result<()> {
        match command {
            PhysicsCommand::SetSimulationSubstepCount(to) => {
                self.set_simulation_substep_count(to);
            }
            PhysicsCommand::SetSimulationSpeed(to) => {
                self.set_simulation_speed(to);
            }
        }
        Ok(())
    }

    pub fn execute_scene_command(&self, command: SceneCommand) -> Result<()> {
        match command {
            SceneCommand::SetSkybox(skybox) => {
                self.set_skybox(skybox);
            }
            SceneCommand::SetSceneEntityActiveState { entity, state } => {
                self.set_scene_entity_active_state(entity, state)?;
            }
        }
        Ok(())
    }

    pub fn execute_control_command(&self, command: ControlCommand) -> Result<()> {
        match command {
            ControlCommand::SetMotion { state, direction } => {
                self.set_motion(state, direction);
            }
            ControlCommand::StopMotion => {
                self.stop_motion();
            }
            ControlCommand::SetMovementSpeed(speed) => {
                self.set_movement_speed(speed);
            }
        }
        Ok(())
    }

    pub fn execute_ui_command(&self, command: UICommand) -> Result<()> {
        match command {
            UICommand::SetInteractionMode(to) => {
                self.set_interaction_mode(to);
            }
        }
        Ok(())
    }

    pub fn execute_capture_command(&self, command: CaptureCommand) -> Result<()> {
        match command {
            CaptureCommand::SaveScreenshot => {
                self.save_screenshot();
            }
            CaptureCommand::SaveShadowMaps(save_for) => {
                self.save_shadow_maps(save_for);
            }
        }
        Ok(())
    }

    // Rendering

    pub fn set_ambient_occlusion(&self, to: ToActiveState) -> ModifiedActiveState {
        log::debug!("Setting ambient occlusion to {to:?}");
        self.renderer.read().unwrap().set_ambient_occlusion(to)
    }

    pub fn set_temporal_anti_aliasing(&self, to: ToActiveState) -> ModifiedActiveState {
        log::debug!("Setting temporal anti-aliasing to {to:?}");
        let renderer = self.renderer().read().unwrap();

        let state = renderer.set_temporal_anti_aliasing(to);

        if state.changed {
            let scene = self.scene().read().unwrap();
            let mut scene_camera = scene.scene_camera().write().unwrap();

            if let Some(camera) = scene_camera.as_mut() {
                camera.set_jitter_enabled(state.state.is_enabled());
                renderer.declare_render_resources_desynchronized();
            }
        }
        state
    }

    pub fn set_bloom(&self, to: ToActiveState) -> ModifiedActiveState {
        log::debug!("Setting bloom to {to:?}");
        self.renderer.read().unwrap().set_bloom(to)
    }

    pub fn set_tone_mapping_method(&self, to: ToToneMappingMethod) -> ToneMappingMethod {
        log::debug!("Setting tone mapping method to {to:?}");
        self.renderer.read().unwrap().set_tone_mapping_method(to)
    }

    pub fn set_exposure(&self, to: ToExposure) {
        log::debug!("Setting exposure to {to:?}");
        self.renderer.read().unwrap().set_exposure(to);
    }

    pub fn set_render_attachment_visualization(&self, to: ToActiveState) -> ModifiedActiveState {
        log::debug!("Setting render attachment visualization to {to:?}");
        self.renderer
            .read()
            .unwrap()
            .set_render_attachment_visualization(to)
    }

    pub fn set_visualized_render_attachment_quantity(
        &self,
        to: ToRenderAttachmentQuantity,
    ) -> Result<RenderAttachmentQuantity> {
        log::debug!("Setting visualized render attachment quantity to {to:?}");
        self.renderer
            .read()
            .unwrap()
            .set_visualized_render_attachment_quantity(to)
    }

    pub fn set_shadow_mapping(&self, to: ToActiveState) -> ModifiedActiveState {
        log::debug!("Setting shadow mapping to {to:?}");
        self.renderer.write().unwrap().set_shadow_mapping(to)
    }

    pub fn set_wireframe_mode(&self, to: ToActiveState) -> ModifiedActiveState {
        log::debug!("Setting wireframe mode to {to:?}");
        self.renderer.write().unwrap().set_wireframe_mode(to)
    }

    pub fn set_render_pass_timings(&self, to: ToActiveState) -> ModifiedActiveState {
        log::debug!("Setting render pass timings to {to:?}");
        self.renderer.write().unwrap().set_render_pass_timings(to)
    }

    // Physics

    pub fn set_simulation_substep_count(&self, to: ToSubstepCount) -> u32 {
        log::debug!("Setting simulation substep count to {to:?}");
        self.simulator
            .write()
            .unwrap()
            .set_simulation_substep_count(to)
    }

    pub fn set_simulation_speed(&self, to: ToSimulationSpeedMultiplier) -> f64 {
        log::debug!("Setting simulation speed to {to:?}");
        let mut simulator = self.simulator.write().unwrap();
        let old_multiplier = simulator.simulation_speed_multiplier();
        let new_multiplier = simulator.set_simulation_speed(to);
        drop(simulator);

        if new_multiplier != old_multiplier {
            // Adjust movement speed to compensate for the change in simulation speed
            if let Some(motion_controller) = &self.motion_controller {
                let mut motion_controller = motion_controller.lock().unwrap();
                let new_movement_speed =
                    motion_controller.movement_speed() * (old_multiplier / new_multiplier);
                motion_controller.set_movement_speed(new_movement_speed);
            }
        }

        new_multiplier
    }

    // Scene

    pub fn set_skybox(&self, skybox: Skybox) {
        log::debug!("Setting skybox to {skybox:?}");
        self.scene().read().unwrap().set_skybox(skybox);

        self.renderer()
            .read()
            .unwrap()
            .declare_render_resources_desynchronized();
    }

    pub fn set_scene_entity_active_state(&self, entity: Entity, state: ActiveState) -> Result<()> {
        log::debug!(
            "Setting state of scene entity {entity} to {state:?}",
            entity = entity.as_u64()
        );
        match state {
            ActiveState::Enabled => self.enable_scene_entity(&entity),
            ActiveState::Disabled => self.disable_scene_entity(&entity),
        }
    }

    // Control

    pub fn set_motion(&self, state: MotionState, direction: MotionDirection) {
        if self.control_mode_active() {
            if let Some(motion_controller) = &self.motion_controller {
                log::debug!("Setting motion in direction {direction:?} to {state:?}");
                motion_controller
                    .lock()
                    .unwrap()
                    .update_motion(state, direction);
            } else {
                log::debug!("Not setting motion since there is no motion controller");
            }
        } else {
            log::debug!("Not setting motion since control mode is disabled");
        }
    }

    pub fn stop_motion(&self) {
        if let Some(motion_controller) = &self.motion_controller {
            log::debug!("Stopping controller motion");
            motion_controller.lock().unwrap().stop();
        } else {
            log::debug!("Not stopping motion since there is no motion controller");
        }
    }

    pub fn set_movement_speed(&self, speed: fph) {
        if let Some(motion_controller) = &self.motion_controller {
            log::debug!("Setting movement speed to {speed:?}");
            motion_controller.lock().unwrap().set_movement_speed(speed);
        } else {
            log::debug!("Not setting movement speed since there is no motion controller");
        }
    }

    // UI

    pub fn set_interaction_mode(&self, to: ToInteractionMode) {
        log::debug!("Setting interaction mode to {to:?}");
        let mut user_interface = self.user_interface().write().unwrap();
        let was_control = user_interface.control_mode_active();
        user_interface.set_interaction_mode(to);
        let is_control = user_interface.control_mode_active();

        if was_control && !is_control {
            self.stop_motion();
        }
    }

    // Capture

    pub fn save_screenshot(&self) {
        log::debug!("Saving screenshot");
        self.screen_capturer().request_screenshot_save();
    }

    pub fn save_shadow_maps(&self, save_for: SaveShadowMapsFor) {
        log::debug!("Saving shadow maps for {save_for:?}");
        match save_for {
            SaveShadowMapsFor::OmnidirectionalLight => {
                self.screen_capturer()
                    .request_omnidirectional_light_shadow_map_save();
            }
            SaveShadowMapsFor::UnidirectionalLight => {
                self.screen_capturer()
                    .request_unidirectional_light_shadow_map_save();
            }
        }
    }
}

impl ToActiveState {
    pub fn set(self, enabled: &mut bool) -> ModifiedActiveState {
        let was_enabled = *enabled;
        let state = self.apply(enabled);
        let changed = *enabled != was_enabled;
        ModifiedActiveState { state, changed }
    }

    fn apply(self, enabled: &mut bool) -> ActiveState {
        match (self, *enabled) {
            (Self::Enabled, _) | (Self::Opposite, false) => {
                *enabled = true;
                ActiveState::Enabled
            }
            (Self::Disabled, _) | (Self::Opposite, true) => {
                *enabled = false;
                ActiveState::Disabled
            }
        }
    }
}

impl ActiveState {
    pub fn is_enabled(self) -> bool {
        self == Self::Enabled
    }
}
