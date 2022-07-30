//! Container for all data in the world.

use crate::{
    control::{MotionController, MotionDirection, MotionState},
    geometry::GeometricalData,
    rendering::RenderingSystem,
    window::ControlFlow,
};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    geometrical_data: GeometricalData,
    renderer: RenderingSystem,
    motion_controller: Box<dyn MotionController<f32>>,
}

impl World {
    /// Creates a new world data container.
    pub fn new(
        geometrical_data: GeometricalData,
        renderer: RenderingSystem,
        controller: impl 'static + MotionController<f32>,
    ) -> Self {
        Self {
            geometrical_data,
            renderer,
            motion_controller: Box::new(controller),
        }
    }

    /// Sets a new size for the rendering surface.
    pub fn resize_rendering_surface(&mut self, new_size: (u32, u32)) {
        self.renderer.resize_surface(new_size);
    }

    /// Instructs the [`RenderingSystem`] to render a frame.
    pub fn render(&mut self, control_flow: &mut ControlFlow<'_>) {
        match self.renderer.render() {
            Ok(_) => {}
            Err(err) => match err.downcast_ref() {
                // Recreate swap chain if lost
                Some(wgpu::SurfaceError::Lost) => self.renderer.initialize_surface(),
                // Quit if GPU is out of memory
                Some(wgpu::SurfaceError::OutOfMemory) => {
                    control_flow.exit();
                }
                // Other errors should be resolved by the next frame, so we just log the error and continue
                _ => log::error!("{:?}", err),
            },
        }
    }

    /// Updates the motion controller with the given motion.
    pub fn update_motion_controller(&mut self, state: MotionState, direction: MotionDirection) {
        self.motion_controller.update_motion(state, direction);
        if let Some(translation) = self.motion_controller.next_translation() {
            self.geometrical_data.transform_cameras(&translation.into());
        }
    }

    /// Propagates the current geometrical data to the rendering system.
    pub fn sync_render_data(&mut self) {
        self.renderer.sync_with_geometry(&mut self.geometrical_data);
    }
}
