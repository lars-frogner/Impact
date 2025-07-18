//! Commands for operating the engine.

use super::Engine;
use crate::{
    command::{ActiveState, ModifiedActiveState, ToActiveState},
    control::{
        command::ControlCommand,
        motion::{MotionDirection, MotionState},
    },
    gpu::rendering::{
        command::RenderingCommand,
        postprocessing::command::{
            PostprocessingCommand, ToExposure, ToRenderAttachmentQuantity, ToToneMappingMethod,
        },
        screen_capture::command::{CaptureCommand, SaveShadowMapsFor},
    },
    instrumentation::command::InstrumentationCommand,
    physics::command::{
        PhysicsCommand, ToSimulationSpeedMultiplier, ToSubstepCount, set_simulation_speed,
        set_simulation_substep_count,
    },
    scene::command::SceneCommand,
};
use anyhow::Result;
use impact_ecs::world::EntityID;
use impact_physics::{fph, medium::UniformMedium};
use impact_rendering::{
    attachment::RenderAttachmentQuantity,
    postprocessing::capturing::dynamic_range_compression::ToneMappingMethod,
};
use impact_scene::skybox::Skybox;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineCommand {
    Rendering(RenderingCommand),
    Physics(PhysicsCommand),
    Scene(SceneCommand),
    Control(ControlCommand),
    Capture(CaptureCommand),
    Instrumentation(InstrumentationCommand),
    Shutdown,
}

impl Engine {
    pub fn execute_command(&self, command: EngineCommand) -> Result<()> {
        match command {
            EngineCommand::Rendering(command) => self.execute_rendering_command(command),
            EngineCommand::Physics(command) => self.execute_physics_command(command),
            EngineCommand::Scene(command) => self.execute_scene_command(command),
            EngineCommand::Control(command) => self.execute_control_command(command),
            EngineCommand::Capture(command) => self.execute_capture_command(command),
            EngineCommand::Instrumentation(command) => {
                self.execute_instrumentation_command(command)
            }
            EngineCommand::Shutdown => {
                self.request_shutdown();
                Ok(())
            }
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
            PhysicsCommand::SetSimulation(to) => {
                self.set_simulation(to);
            }
            PhysicsCommand::SetSimulationSubstepCount(to) => {
                self.set_simulation_substep_count(to);
            }
            PhysicsCommand::SetSimulationSpeed(to) => {
                self.set_simulation_speed(to);
            }
            PhysicsCommand::SetMedium(to) => {
                self.set_medium(to);
            }
        }
        Ok(())
    }

    pub fn execute_scene_command(&self, command: SceneCommand) -> Result<()> {
        match command {
            SceneCommand::SetSkybox(skybox) => {
                self.set_skybox(skybox);
            }
            SceneCommand::SetSceneEntityActiveState {
                entity_id: entity,
                state,
            } => {
                self.set_scene_entity_active_state(entity, state)?;
            }
            SceneCommand::Clear => self.reset_world(),
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

    pub fn execute_capture_command(&self, command: CaptureCommand) -> Result<()> {
        match command {
            CaptureCommand::SaveScreenshot => {
                self.request_screenshot_save();
            }
            CaptureCommand::SaveShadowMaps(save_for) => {
                self.request_shadow_map_saves(save_for);
            }
        }
        Ok(())
    }

    pub fn execute_instrumentation_command(&self, command: InstrumentationCommand) -> Result<()> {
        match command {
            InstrumentationCommand::SetTaskTimings(to) => {
                self.set_task_timings(to);
            }
        }
        Ok(())
    }

    // Rendering

    pub fn set_ambient_occlusion(&self, to: ToActiveState) -> ModifiedActiveState {
        impact_log::info!("Setting ambient occlusion to {to:?}");
        self.renderer.read().unwrap().set_ambient_occlusion(to)
    }

    pub fn set_temporal_anti_aliasing(&self, to: ToActiveState) -> ModifiedActiveState {
        impact_log::info!("Setting temporal anti-aliasing to {to:?}");
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
        impact_log::info!("Setting bloom to {to:?}");
        self.renderer.read().unwrap().set_bloom(to)
    }

    pub fn set_tone_mapping_method(&self, to: ToToneMappingMethod) -> ToneMappingMethod {
        impact_log::info!("Setting tone mapping method to {to:?}");
        self.renderer.read().unwrap().set_tone_mapping_method(to)
    }

    pub fn set_exposure(&self, to: ToExposure) {
        impact_log::info!("Setting exposure to {to:?}");
        self.renderer.read().unwrap().set_exposure(to);
    }

    pub fn set_render_attachment_visualization(&self, to: ToActiveState) -> ModifiedActiveState {
        impact_log::info!("Setting render attachment visualization to {to:?}");
        self.renderer
            .read()
            .unwrap()
            .set_render_attachment_visualization(to)
    }

    pub fn set_visualized_render_attachment_quantity(
        &self,
        to: ToRenderAttachmentQuantity,
    ) -> Result<RenderAttachmentQuantity> {
        impact_log::info!("Setting visualized render attachment quantity to {to:?}");
        self.renderer
            .read()
            .unwrap()
            .set_visualized_render_attachment_quantity(to)
    }

    pub fn set_shadow_mapping(&self, to: ToActiveState) -> ModifiedActiveState {
        impact_log::info!("Setting shadow mapping to {to:?}");
        self.renderer.write().unwrap().set_shadow_mapping(to)
    }

    pub fn set_wireframe_mode(&self, to: ToActiveState) -> ModifiedActiveState {
        impact_log::info!("Setting wireframe mode to {to:?}");
        self.renderer.write().unwrap().set_wireframe_mode(to)
    }

    pub fn set_render_pass_timings(&self, to: ToActiveState) -> ModifiedActiveState {
        impact_log::info!("Setting render pass timings to {to:?}");
        self.renderer.write().unwrap().set_render_pass_timings(to)
    }

    // Physics

    pub fn set_simulation(&self, to: ToActiveState) -> ModifiedActiveState {
        impact_log::info!("Setting simulation to {to:?}");
        let mut simulator = self.simulator.write().unwrap();
        to.set(simulator.enabled_mut())
    }

    pub fn set_simulation_substep_count(&self, to: ToSubstepCount) -> u32 {
        impact_log::info!("Setting simulation substep count to {to:?}");
        set_simulation_substep_count(&mut self.simulator.write().unwrap(), to)
    }

    pub fn set_simulation_speed(&self, to: ToSimulationSpeedMultiplier) -> f64 {
        impact_log::info!("Setting simulation speed to {to:?}");
        let mut simulator = self.simulator.write().unwrap();
        let old_multiplier = simulator.simulation_speed_multiplier();
        let new_multiplier = set_simulation_speed(&mut simulator, to);
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

    pub fn set_medium(&self, to: UniformMedium) {
        self.simulator.write().unwrap().set_medium(to);
    }

    // Scene

    pub fn set_skybox(&self, skybox: Skybox) {
        impact_log::info!("Setting skybox to {skybox:?}");
        self.scene().read().unwrap().set_skybox(skybox);

        self.renderer()
            .read()
            .unwrap()
            .declare_render_resources_desynchronized();
    }

    pub fn set_scene_entity_active_state(
        &self,
        entity_id: EntityID,
        state: ActiveState,
    ) -> Result<()> {
        impact_log::info!("Setting state of scene entity with ID {entity_id} to {state:?}");
        match state {
            ActiveState::Enabled => self.enable_scene_entity(entity_id),
            ActiveState::Disabled => self.disable_scene_entity(entity_id),
        }
    }

    // Control

    pub fn set_motion(&self, state: MotionState, direction: MotionDirection) {
        if self.controls_enabled() {
            if let Some(motion_controller) = &self.motion_controller {
                impact_log::debug!("Setting motion in direction {direction:?} to {state:?}");
                motion_controller
                    .lock()
                    .unwrap()
                    .update_motion(state, direction);
            } else {
                impact_log::info!("Not setting motion since there is no motion controller");
            }
        } else {
            impact_log::info!("Not setting motion since controls are disabled");
        }
    }

    pub fn stop_motion(&self) {
        if let Some(motion_controller) = &self.motion_controller {
            impact_log::info!("Stopping controller motion");
            motion_controller.lock().unwrap().stop();
        } else {
            impact_log::info!("Not stopping motion since there is no motion controller");
        }
    }

    pub fn set_movement_speed(&self, speed: fph) {
        if let Some(motion_controller) = &self.motion_controller {
            impact_log::info!("Setting movement speed to {speed:?}");
            motion_controller.lock().unwrap().set_movement_speed(speed);
        } else {
            impact_log::info!("Not setting movement speed since there is no motion controller");
        }
    }

    // Capture

    pub fn request_screenshot_save(&self) {
        impact_log::info!("Requesting screenshot save");
        self.screen_capturer().request_screenshot_save();
    }

    pub fn request_shadow_map_saves(&self, save_for: SaveShadowMapsFor) {
        impact_log::info!("Requesting shadow map saves for {save_for:?}");
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

    // Instrumentation

    pub fn set_task_timings(&self, to: ToActiveState) {
        let mut enabled = self.task_timer.enabled();
        if to.set(&mut enabled).changed {
            self.task_timer.set_enabled(enabled);
        }
    }
}
