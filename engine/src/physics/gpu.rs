//! GPU simulation.

use crate::{gpu::GraphicsSystem, lock_order::OrderedRwLock, resource::ResourceManager};
use anyhow::Result;
use impact_gpu::wgpu;
use impact_particle::simulation_commands::ParticleSimulationCommands;

/// Manager of GPU commands for physics simulation.
#[derive(Debug)]
pub struct GPUSimulationManager {
    particle_simulation_commands: Option<ParticleSimulationCommands>,
}

impl GPUSimulationManager {
    /// Creates a new manager with initialized physics simulation GPU commands.
    pub fn new(graphics_system: &GraphicsSystem, resource_manager: &ResourceManager) -> Self {
        let mut shader_manager = graphics_system.shader_manager().owrite();

        let particle_simulation_commands = ParticleSimulationCommands::new(
            graphics_system.graphics_device(),
            &mut shader_manager,
            resource_manager,
            graphics_system.bind_group_layout_registry(),
        );

        Self {
            particle_simulation_commands,
        }
    }

    /// Synchronizes the simulation commands with the current GPU resources.
    ///
    /// # Errors
    /// Returns an error if synchronization fails due to missing resources.
    pub fn synchronize_simulation_commands(
        &mut self,
        graphics_system: &GraphicsSystem,
    ) -> Result<()> {
        if self.has_nothing_to_simulate() {
            return Ok(());
        }

        let mut shader_manager = graphics_system.shader_manager().owrite();
        let gpu_resource_manager = graphics_system.gpu_resource_manager().oread();
        let gpu_resource_manager = &**gpu_resource_manager;

        if let Some(particle_simulation_commands) = &mut self.particle_simulation_commands {
            particle_simulation_commands.sync_with_gpu_resources(
                graphics_system.graphics_device(),
                &mut shader_manager,
                gpu_resource_manager,
            )?;
        }
        Ok(())
    }

    pub fn simulate_for_time_step(
        &self,
        graphics_system: &GraphicsSystem,
        current_time_step: u64,
    ) -> Result<()> {
        if self.has_nothing_to_simulate() {
            return Ok(());
        }

        let mut command_encoder =
            Self::create_simulation_command_encoder(graphics_system.graphics_device().device());

        {
            let mut timestamp_query_manager = graphics_system.timestamp_query_manager().owrite();
            let mut timestamp_recorder = timestamp_query_manager.create_timestamp_query_registry();

            let gpu_resource_manager = graphics_system.gpu_resource_manager().oread();
            let gpu_resource_manager = &**gpu_resource_manager;

            if let Some(particle_simulation_commands) = &self.particle_simulation_commands {
                particle_simulation_commands.record(
                    gpu_resource_manager,
                    &mut timestamp_recorder,
                    &mut command_encoder,
                    current_time_step,
                )?;
            }

            timestamp_recorder.finish(&mut command_encoder);
        }

        graphics_system
            .graphics_device()
            .queue()
            .submit(std::iter::once(command_encoder.finish()));

        Ok(())
    }

    fn has_nothing_to_simulate(&self) -> bool {
        self.particle_simulation_commands.is_none()
    }

    fn create_simulation_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Simulation command encoder"),
        })
    }
}
