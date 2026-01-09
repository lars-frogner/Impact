//! Engine commands.

pub mod capture;
pub mod controller;
pub mod game_loop;
pub mod gizmo;
pub mod instrumentation;
pub mod physics;
pub mod queue;
pub mod rendering;
pub mod scene;
pub mod uils;

use crate::{
    command::{controller::ControlAdminCommand, queue::CommandQueue},
    engine::Engine,
    lock_order::OrderedRwLock,
};
use anyhow::Result;
use capture::CaptureAdminCommand;
use controller::ControlCommand;
use game_loop::GameLoopAdminCommand;
use gizmo::GizmoAdminCommand;
use instrumentation::InstrumentationAdminCommand;
use physics::PhysicsAdminCommand;
use rendering::RenderingAdminCommand;
use roc_integration::roc;
use scene::SceneCommand;

#[roc(name = "EngineCommand", parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserCommand {
    Scene(SceneCommand),
    Controller(ControlCommand),
}

#[derive(Clone, Debug)]
pub enum AdminCommand {
    Rendering(RenderingAdminCommand),
    Physics(PhysicsAdminCommand),
    Control(ControlAdminCommand),
    Capture(CaptureAdminCommand),
    Instrumentation(InstrumentationAdminCommand),
    GameLoop(GameLoopAdminCommand),
    Gizmo(GizmoAdminCommand),
    System(SystemAdminCommand),
}

#[derive(Clone, Debug)]
pub enum SystemAdminCommand {
    ResetWorld,
    Shutdown,
}

#[derive(Debug, Default)]
pub struct EngineCommandQueues {
    // User commands
    pub scene: CommandQueue<SceneCommand>,
    pub controller: CommandQueue<ControlCommand>,
    // Admin commands
    pub rendering: CommandQueue<RenderingAdminCommand>,
    pub physics: CommandQueue<PhysicsAdminCommand>,
    pub control: CommandQueue<ControlAdminCommand>,
    pub capture: CommandQueue<CaptureAdminCommand>,
    pub instrumentation: CommandQueue<InstrumentationAdminCommand>,
    pub game_loop: CommandQueue<GameLoopAdminCommand>,
    pub gizmo: CommandQueue<GizmoAdminCommand>,
    pub system: CommandQueue<SystemAdminCommand>,
}

pub fn execute_engine_command(engine: &Engine, command: UserCommand) -> Result<()> {
    match command {
        UserCommand::Scene(command) => execute_scene_command(engine, command),
        UserCommand::Controller(command) => execute_control_command(engine, command),
    }
}

pub fn execute_admin_command(engine: &Engine, command: AdminCommand) -> Result<()> {
    match command {
        AdminCommand::Rendering(command) => execute_rendering_admin_command(engine, command),
        AdminCommand::Physics(command) => execute_physics_admin_command(engine, command),
        AdminCommand::Control(command) => execute_control_admin_command(engine, command),
        AdminCommand::Capture(command) => execute_capture_admin_command(engine, command),
        AdminCommand::Instrumentation(command) => {
            execute_instrumentation_admin_command(engine, command)
        }
        AdminCommand::GameLoop(command) => execute_game_loop_admin_command(engine, command),
        AdminCommand::Gizmo(command) => execute_gizmo_admin_command(engine, command),
        AdminCommand::System(command) => execute_system_admin_command(engine, command),
    }
}

pub fn execute_scene_command(engine: &Engine, command: SceneCommand) -> Result<()> {
    match command {
        SceneCommand::SetSkybox(skybox) => {
            scene::set_skybox(engine, skybox);
        }
        SceneCommand::SetMedium(medium) => {
            scene::set_medium(engine, medium);
        }
        SceneCommand::SetSceneEntityActiveState { entity_id, state } => {
            scene::set_scene_entity_active_state(engine, entity_id, state)?;
        }
    }
    Ok(())
}

pub fn execute_control_command(engine: &Engine, command: ControlCommand) -> Result<()> {
    match command {
        ControlCommand::SetMotion { state, direction } => {
            controller::set_motion(engine, state, direction);
        }
        ControlCommand::StopMotion => {
            controller::stop_motion(engine);
        }
        ControlCommand::SetMovementSpeed(speed) => {
            controller::set_movement_speed(engine, speed);
        }
    }
    Ok(())
}

pub fn execute_rendering_admin_command(
    engine: &Engine,
    command: RenderingAdminCommand,
) -> Result<()> {
    match command {
        RenderingAdminCommand::SetAmbientOcclusion(to) => {
            rendering::set_ambient_occlusion(&engine.renderer().oread(), to);
        }
        RenderingAdminCommand::SetTemporalAntiAliasing(to) => {
            rendering::set_temporal_anti_aliasing(engine.scene(), engine.renderer(), to);
        }
        RenderingAdminCommand::SetBloom(to) => {
            rendering::set_bloom(&engine.renderer().oread(), to);
        }
        RenderingAdminCommand::SetToneMappingMethod(to) => {
            rendering::set_tone_mapping_method(&engine.renderer().oread(), to);
        }
        RenderingAdminCommand::SetExposure(to) => {
            rendering::set_exposure(&engine.renderer().oread(), to);
        }
        RenderingAdminCommand::SetRenderAttachmentVisualization(to) => {
            rendering::set_render_attachment_visualization(&engine.renderer().oread(), to);
        }
        RenderingAdminCommand::SetVisualizedRenderAttachmentQuantity(to) => {
            rendering::set_visualized_render_attachment_quantity(&engine.renderer().oread(), to)?;
        }
        RenderingAdminCommand::SetShadowMapping(to) => {
            rendering::set_shadow_mapping(&mut engine.renderer().owrite(), to);
        }
        RenderingAdminCommand::SetWireframeMode(to) => {
            rendering::set_wireframe_mode(&mut engine.renderer().owrite(), to);
        }
        RenderingAdminCommand::SetRenderPassTimings(to) => {
            rendering::set_render_pass_timings(&mut engine.renderer().owrite(), to);
        }
        RenderingAdminCommand::SetAmbientOcclusionConfig(config) => {
            rendering::set_ambient_occlusion_config(&engine.renderer().oread(), config);
        }
        RenderingAdminCommand::SetTemporalAntiAliasingConfig(config) => {
            rendering::set_temporal_anti_aliasing_config(&engine.renderer().oread(), config);
        }
        RenderingAdminCommand::SetBloomConfig(config) => {
            rendering::set_bloom_config(&engine.renderer().oread(), config);
        }
        RenderingAdminCommand::SetCameraSettings(settings) => {
            rendering::set_camera_settings(&engine.renderer().oread(), settings);
        }
        RenderingAdminCommand::SetAverageLuminanceComputationConfig(config) => {
            rendering::set_average_luminance_computation_config(&engine.renderer().oread(), config);
        }
        RenderingAdminCommand::SetDynamicRangeCompressionConfig(config) => {
            rendering::set_dynamic_range_compression_config(&engine.renderer().oread(), config);
        }
    }
    Ok(())
}

pub fn execute_physics_admin_command(engine: &Engine, command: PhysicsAdminCommand) -> Result<()> {
    match command {
        PhysicsAdminCommand::SetSimulation(to) => {
            physics::set_simulation(&mut engine.simulator().owrite(), to);
        }
        PhysicsAdminCommand::SetSimulationSubstepCount(to) => {
            physics::set_simulation_substep_count(&mut engine.simulator().owrite(), to);
        }
        PhysicsAdminCommand::SetSimulationSpeed(to) => {
            physics::set_simulation_speed_and_compensate_controller_movement_speed(engine, to);
        }
        PhysicsAdminCommand::SetTimeStepDuration(duration) => {
            physics::set_time_step_duration(&mut engine.simulator().owrite(), duration);
        }
        PhysicsAdminCommand::SetMatchFrameDuration(to) => {
            physics::set_match_frame_duration(&mut engine.simulator().owrite(), to);
        }
        PhysicsAdminCommand::SetConstraintSolverConfig(config) => {
            physics::set_constraint_solver_config(&mut engine.simulator().owrite(), config);
        }
    }
    Ok(())
}

pub fn execute_control_admin_command(engine: &Engine, command: ControlAdminCommand) -> Result<()> {
    match command {
        ControlAdminCommand::SetControls(to) => {
            engine.set_controls_enabled(to.enabled());
        }
    }
    Ok(())
}

pub fn execute_capture_admin_command(engine: &Engine, command: CaptureAdminCommand) -> Result<()> {
    match command {
        CaptureAdminCommand::SaveScreenshot => {
            capture::request_screenshot_save(engine.screen_capturer());
        }
        CaptureAdminCommand::SaveShadowMaps(save_for) => {
            capture::request_shadow_map_saves(engine.screen_capturer(), save_for);
        }
    }
    Ok(())
}

pub fn execute_instrumentation_admin_command(
    engine: &Engine,
    command: InstrumentationAdminCommand,
) -> Result<()> {
    match command {
        InstrumentationAdminCommand::SetTaskTimings(to) => {
            instrumentation::set_task_timings(engine.task_timer(), to);
        }
        InstrumentationAdminCommand::SetRenderPassTimings(to) => {
            instrumentation::set_render_pass_timings(&mut engine.renderer().owrite(), to);
        }
    }
    Ok(())
}

pub fn execute_game_loop_admin_command(
    engine: &Engine,
    command: GameLoopAdminCommand,
) -> Result<()> {
    let mut game_loop_controller = engine.game_loop_controller().owrite();
    match command {
        GameLoopAdminCommand::SetGameLoop(to) => {
            game_loop::set_game_loop(&mut game_loop_controller, to);
        }
    }
    Ok(())
}

pub fn execute_gizmo_admin_command(engine: &Engine, command: GizmoAdminCommand) -> Result<()> {
    match command {
        GizmoAdminCommand::SetVisibility {
            gizmo_type,
            visibility,
        } => {
            gizmo::set_gizmo_visibility(engine, gizmo_type, visibility);
        }
        GizmoAdminCommand::SetParameters(parameters) => {
            gizmo::set_gizmo_parameters(engine, parameters);
        }
    }
    Ok(())
}

pub fn execute_system_admin_command(engine: &Engine, command: SystemAdminCommand) -> Result<()> {
    match command {
        SystemAdminCommand::ResetWorld => engine.reset_world(),
        SystemAdminCommand::Shutdown => engine.request_shutdown(),
    }
    Ok(())
}
