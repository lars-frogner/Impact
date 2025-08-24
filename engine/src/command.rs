//! Engine commands.

pub mod capture;
pub mod controller;
pub mod game_loop;
pub mod instrumentation;
pub mod physics;
pub mod queue;
pub mod rendering;
pub mod scene;
pub mod uils;

use crate::{
    engine::Engine,
    lock_order::{OrderedMutex, OrderedRwLock},
};
use anyhow::Result;
use capture::CaptureCommand;
use controller::ControllerCommand;
use game_loop::GameLoopCommand;
use instrumentation::InstrumentationCommand;
use physics::PhysicsCommand;
use rendering::RenderingCommand;
use roc_integration::roc;
use scene::SceneCommand;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineCommand {
    Rendering(RenderingCommand),
    Physics(PhysicsCommand),
    Scene(SceneCommand),
    Controller(ControllerCommand),
    Capture(CaptureCommand),
    Instrumentation(InstrumentationCommand),
    GameLoop(GameLoopCommand),
    Shutdown,
}

pub fn execute_engine_command(engine: &Engine, command: EngineCommand) -> Result<()> {
    match command {
        EngineCommand::Rendering(command) => execute_rendering_command(engine, command),
        EngineCommand::Physics(command) => execute_physics_command(engine, command),
        EngineCommand::Scene(command) => execute_scene_command(engine, command),
        EngineCommand::Controller(command) => execute_control_command(engine, command),
        EngineCommand::Capture(command) => execute_capture_command(engine, command),
        EngineCommand::Instrumentation(command) => execute_instrumentation_command(engine, command),
        EngineCommand::GameLoop(command) => execute_game_loop_command(engine, command),
        EngineCommand::Shutdown => {
            engine.request_shutdown();
            Ok(())
        }
    }
}

pub fn execute_rendering_command(engine: &Engine, command: RenderingCommand) -> Result<()> {
    match command {
        RenderingCommand::SetAmbientOcclusion(to) => {
            rendering::set_ambient_occlusion(&engine.renderer().oread(), to);
        }
        RenderingCommand::SetTemporalAntiAliasing(to) => {
            rendering::set_temporal_anti_aliasing(engine.scene(), engine.renderer(), to);
        }
        RenderingCommand::SetBloom(to) => {
            rendering::set_bloom(&engine.renderer().oread(), to);
        }
        RenderingCommand::SetToneMappingMethod(to) => {
            rendering::set_tone_mapping_method(&engine.renderer().oread(), to);
        }
        RenderingCommand::SetExposure(to) => {
            rendering::set_exposure(&engine.renderer().oread(), to);
        }
        RenderingCommand::SetRenderAttachmentVisualization(to) => {
            rendering::set_render_attachment_visualization(&engine.renderer().oread(), to);
        }
        RenderingCommand::SetVisualizedRenderAttachmentQuantity(to) => {
            rendering::set_visualized_render_attachment_quantity(&engine.renderer().oread(), to)?;
        }
        RenderingCommand::SetShadowMapping(to) => {
            rendering::set_shadow_mapping(&mut engine.renderer().owrite(), to);
        }
        RenderingCommand::SetWireframeMode(to) => {
            rendering::set_wireframe_mode(&mut engine.renderer().owrite(), to);
        }
        RenderingCommand::SetRenderPassTimings(to) => {
            rendering::set_render_pass_timings(&mut engine.renderer().owrite(), to);
        }
    }
    Ok(())
}

pub fn execute_physics_command(engine: &Engine, command: PhysicsCommand) -> Result<()> {
    match command {
        PhysicsCommand::SetSimulation(to) => {
            physics::set_simulation(&mut engine.simulator().owrite(), to);
        }
        PhysicsCommand::SetSimulationSubstepCount(to) => {
            physics::set_simulation_substep_count(&mut engine.simulator().owrite(), to);
        }
        PhysicsCommand::SetSimulationSpeed(to) => {
            physics::set_simulation_speed_and_compensate_controller_movement_speed(engine, to);
        }
        PhysicsCommand::SetMedium(to) => {
            physics::set_medium(&mut engine.simulator().owrite(), to);
        }
    }
    Ok(())
}

pub fn execute_scene_command(engine: &Engine, command: SceneCommand) -> Result<()> {
    match command {
        SceneCommand::SetSkybox(skybox) => {
            scene::set_skybox(engine, skybox);
        }
        SceneCommand::SetSceneEntityActiveState { entity_id, state } => {
            scene::set_scene_entity_active_state(engine, entity_id, state)?;
        }
        SceneCommand::Clear => {
            scene::clear(engine);
        }
    }
    Ok(())
}

pub fn execute_control_command(engine: &Engine, command: ControllerCommand) -> Result<()> {
    match command {
        ControllerCommand::SetMotion { state, direction } => {
            controller::set_motion(engine, state, direction);
        }
        ControllerCommand::StopMotion => {
            controller::stop_motion(engine);
        }
        ControllerCommand::SetMovementSpeed(speed) => {
            controller::set_movement_speed(engine, speed);
        }
    }
    Ok(())
}

pub fn execute_capture_command(engine: &Engine, command: CaptureCommand) -> Result<()> {
    match command {
        CaptureCommand::SaveScreenshot => {
            capture::request_screenshot_save(engine.screen_capturer());
        }
        CaptureCommand::SaveShadowMaps(save_for) => {
            capture::request_shadow_map_saves(engine.screen_capturer(), save_for);
        }
    }
    Ok(())
}

pub fn execute_instrumentation_command(
    engine: &Engine,
    command: InstrumentationCommand,
) -> Result<()> {
    match command {
        InstrumentationCommand::SetTaskTimings(to) => {
            instrumentation::set_task_timings(engine.task_timer(), to);
        }
    }
    Ok(())
}

pub fn execute_game_loop_command(engine: &Engine, command: GameLoopCommand) -> Result<()> {
    let mut game_loop_controller = engine.game_loop_controller().olock();
    match command {
        GameLoopCommand::SetGameLoop(to) => game_loop::set_game_loop(&mut game_loop_controller, to),
        GameLoopCommand::PauseAfterSingleIteration => {
            game_loop::pause_after_single_iteration(&mut game_loop_controller);
        }
    }
    Ok(())
}
