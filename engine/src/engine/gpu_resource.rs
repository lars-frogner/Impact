//! GPU resource synchronization.

use super::Engine;
use crate::lock_order::OrderedRwLock;
use anyhow::Result;

impl Engine {
    pub(crate) fn sync_all_gpu_resources(&self) -> Result<()> {
        self.sync_texture_gpu_resources()?;
        self.sync_mesh_gpu_resources()?;
        self.sync_material_gpu_resources()?;
        self.sync_misc_gpu_resources()?;
        self.sync_dynamic_gpu_resources()?;
        Ok(())
    }

    /// Synchronizes GPU resources for textures.
    pub(crate) fn sync_texture_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let graphics_system = self.graphics_system().oread();

        let mut gpu_resource_manager = graphics_system.gpu_resource_manager().owrite();
        let gpu_resource_manager = &mut **gpu_resource_manager;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(
                graphics_system.graphics_device(),
                graphics_system.mipmapper_generator().as_ref(),
            ),
            &resource_manager.textures,
            &mut gpu_resource_manager.textures,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            graphics_system.graphics_device(),
            &resource_manager.samplers,
            &mut gpu_resource_manager.samplers,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(
                graphics_system.graphics_device(),
                graphics_system.bind_group_layout_registry(),
                &gpu_resource_manager.textures,
                &gpu_resource_manager.samplers,
            ),
            &resource_manager.lookup_tables,
            &mut gpu_resource_manager.lookup_table_bind_groups,
        )?;

        Ok(())
    }

    /// Synchronizes mesh GPU resources for triangle and line segment meshes.
    pub(crate) fn sync_mesh_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let graphics_system = self.graphics_system().oread();
        let mut gpu_resource_manager = graphics_system.gpu_resource_manager().owrite();

        impact_resource::gpu::sync_mutable_gpu_resources(
            graphics_system.graphics_device(),
            &resource_manager.triangle_meshes,
            &mut gpu_resource_manager.triangle_meshes,
        )?;

        impact_resource::gpu::sync_mutable_gpu_resources(
            graphics_system.graphics_device(),
            &resource_manager.line_segment_meshes,
            &mut gpu_resource_manager.line_segment_meshes,
        )?;

        Ok(())
    }

    /// Synchronizes GPU resources for materials.
    pub(crate) fn sync_material_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let graphics_system = self.graphics_system().oread();

        let mut gpu_resource_manager = graphics_system.gpu_resource_manager().owrite();
        let gpu_resource_manager = &mut **gpu_resource_manager;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(),
            &resource_manager.materials,
            &mut gpu_resource_manager.materials,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            graphics_system.graphics_device(),
            &resource_manager.material_templates,
            &mut gpu_resource_manager.material_templates,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(
                graphics_system.graphics_device(),
                &gpu_resource_manager.textures,
                &gpu_resource_manager.samplers,
                &gpu_resource_manager.material_templates,
            ),
            &resource_manager.material_texture_groups,
            &mut gpu_resource_manager.material_texture_groups,
        )?;

        Ok(())
    }

    /// Synchronizes miscellaneous GPU resources.
    pub(crate) fn sync_misc_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let scene = self.scene().oread();
        let skybox = scene.skybox().oread();
        let graphics_system = self.graphics_system().oread();

        let mut gpu_resource_manager = graphics_system.gpu_resource_manager().owrite();
        let gpu_resource_manager = &mut **gpu_resource_manager;

        impact_scene::skybox::sync_gpu_resources_for_skybox(
            skybox.as_ref(),
            graphics_system.graphics_device(),
            &gpu_resource_manager.textures,
            &gpu_resource_manager.samplers,
            &mut gpu_resource_manager.skybox,
        )?;

        resource_manager.particle_materials.sync_gpu_resources(
            graphics_system.graphics_device(),
            &gpu_resource_manager.textures,
            &gpu_resource_manager.samplers,
            graphics_system.bind_group_layout_registry(),
            &mut gpu_resource_manager.particle_materials,
        )?;

        resource_manager.voxel_types.sync_material_gpu_resources(
            graphics_system.graphics_device(),
            &gpu_resource_manager.textures,
            &gpu_resource_manager.samplers,
            graphics_system.bind_group_layout_registry(),
            &mut gpu_resource_manager.voxel_materials,
        )?;

        Ok(())
    }

    /// Records and submits commands for synchronizing dynamic GPU resources
    /// (resources that benefit from a staging belt).
    pub(crate) fn sync_dynamic_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let scene = self.scene().oread();
        let camera_manager = scene.camera_manager().oread();
        let light_manager = scene.light_manager().oread();
        let particle_emitter_manager = scene.particle_emitter_manager().oread();
        let mut voxel_manager = scene.voxel_manager().owrite();
        let mut model_instance_manager = scene.model_instance_manager().owrite();

        let effective_time_step_duration = self.simulator().oread().effective_time_step_duration();

        let graphics_system = self.graphics_system().oread();

        graphics_system.sync_dynamic_gpu_resources(
            &camera_manager,
            &light_manager,
            &resource_manager.particle_materials,
            &particle_emitter_manager,
            voxel_manager.object_manager_mut(),
            &mut model_instance_manager,
            effective_time_step_duration,
        );
        Ok(())
    }
}
