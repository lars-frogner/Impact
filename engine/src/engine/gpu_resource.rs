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
        let renderer = self.renderer().oread();
        let mut render_resource_manager = renderer.render_resource_manager().owrite();
        let render_resource_manager = &mut **render_resource_manager;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(
                self.graphics_device(),
                renderer.mipmapper_generator().as_ref(),
            ),
            &resource_manager.textures,
            &mut render_resource_manager.textures,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            self.graphics_device(),
            &resource_manager.samplers,
            &mut render_resource_manager.samplers,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(
                self.graphics_device(),
                renderer.bind_group_layout_registry(),
                &render_resource_manager.textures,
                &render_resource_manager.samplers,
            ),
            &resource_manager.lookup_tables,
            &mut render_resource_manager.lookup_table_bind_groups,
        )?;

        Ok(())
    }

    /// Synchronizes mesh GPU resources for triangle and line segment meshes.
    pub(crate) fn sync_mesh_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let renderer = self.renderer().oread();
        let mut render_resource_manager = renderer.render_resource_manager().owrite();

        impact_resource::gpu::sync_mutable_gpu_resources(
            self.graphics_device(),
            &resource_manager.triangle_meshes,
            &mut render_resource_manager.triangle_meshes,
        )?;

        impact_resource::gpu::sync_mutable_gpu_resources(
            self.graphics_device(),
            &resource_manager.line_segment_meshes,
            &mut render_resource_manager.line_segment_meshes,
        )?;

        Ok(())
    }

    /// Synchronizes GPU resources for materials.
    pub(crate) fn sync_material_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let renderer = self.renderer().oread();
        let mut render_resource_manager = renderer.render_resource_manager().owrite();
        let render_resource_manager = &mut **render_resource_manager;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(),
            &resource_manager.materials,
            &mut render_resource_manager.materials,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            self.graphics_device(),
            &resource_manager.material_templates,
            &mut render_resource_manager.material_templates,
        )?;

        impact_resource::gpu::sync_immutable_gpu_resources(
            &(
                self.graphics_device(),
                &render_resource_manager.textures,
                &render_resource_manager.samplers,
                &render_resource_manager.material_templates,
            ),
            &resource_manager.material_texture_groups,
            &mut render_resource_manager.material_texture_groups,
        )?;

        Ok(())
    }

    /// Synchronizes miscellaneous GPU resources.
    pub(crate) fn sync_misc_gpu_resources(&self) -> Result<()> {
        let resource_manager = self.resource_manager().oread();
        let scene = self.scene().oread();
        let skybox = scene.skybox().oread();
        let renderer = self.renderer().oread();
        let mut render_resource_manager = renderer.render_resource_manager().owrite();
        let render_resource_manager = &mut **render_resource_manager;

        impact_scene::skybox::sync_gpu_resources_for_skybox(
            skybox.as_ref(),
            renderer.graphics_device(),
            &render_resource_manager.textures,
            &render_resource_manager.samplers,
            &mut render_resource_manager.skybox,
        )?;

        resource_manager.voxel_types.sync_material_gpu_resources(
            renderer.graphics_device(),
            &render_resource_manager.textures,
            &render_resource_manager.samplers,
            renderer.bind_group_layout_registry(),
            &mut render_resource_manager.voxel_materials,
        )?;

        Ok(())
    }

    /// Records and submits commands for synchronizing dynamic GPU resources
    /// (resources that benefit from a staging belt).
    pub(crate) fn sync_dynamic_gpu_resources(&self) -> Result<()> {
        let scene = self.scene().oread();
        let camera_manager = scene.camera_manager().oread();
        let light_manager = scene.light_manager().oread();
        let mut voxel_manager = scene.voxel_manager().owrite();
        let mut model_instance_manager = scene.model_instance_manager().owrite();
        let mut renderer = self.renderer().owrite();

        renderer.sync_dynamic_gpu_resources(
            &camera_manager,
            &light_manager,
            voxel_manager.object_manager_mut(),
            &mut model_instance_manager,
        );
        Ok(())
    }
}
