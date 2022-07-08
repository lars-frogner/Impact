use crate::{
    control::{MotionController, MotionDirection, MotionState},
    geometry::GeometricalData,
    rendering::RenderingSystem,
};

pub struct World {
    geometrical_data: GeometricalData,
    renderer: RenderingSystem,
    motion_controller: Box<dyn MotionController<f32>>,
}

impl World {
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

    pub fn renderer(&self) -> &RenderingSystem {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut RenderingSystem {
        &mut self.renderer
    }

    pub fn update_motion_controller(&mut self, direction: MotionDirection, state: MotionState) {
        self.motion_controller.update_motion(direction, state);
        if let Some(translation) = self.motion_controller.next_translation() {
            self.geometrical_data.transform_cameras(&translation.into());
            self.update();
        }
    }

    fn update(&mut self) {
        self.renderer.sync_with_geometry(&mut self.geometrical_data);
    }
}
