//! Container for all data in the world.

use crate::{
    control::{self, MotionController, MotionDirection, MotionState, OrientationController},
    physics::{OrientationComp, PhysicsSimulator, PositionComp},
    rendering::{MaterialComp, RenderingSystem},
    scene::{
        self as sc, CameraComp, CameraNodeID, MeshComp, ModelID, ModelInstanceNodeID, Scene,
        SceneGraphNodeComp,
    },
    scheduling::TaskScheduler,
    thread::ThreadPoolTaskErrors,
    ui::UserInterface,
    window::{self, ControlFlow, Window},
};
use anyhow::Result;
use impact_ecs::{
    archetype::{ArchetypeCompByteView, ComponentManager},
    setup,
    world::{Entity, World as ECSWorld},
};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex, RwLock},
};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    window: Arc<Window>,
    user_interface: RwLock<UserInterface>,
    ecs_world: RwLock<ECSWorld>,
    scene: RwLock<Scene>,
    renderer: RwLock<RenderingSystem>,
    simulator: RwLock<PhysicsSimulator>,
    motion_controller: Option<Mutex<Box<dyn MotionController>>>,
    orientation_controller: Option<Mutex<Box<dyn OrientationController>>>,
}

pub type WorldTaskScheduler = TaskScheduler<World>;

impl World {
    /// Creates a new world data container.
    pub fn new(
        window: Window,
        scene: Scene,
        renderer: RenderingSystem,
        simulator: PhysicsSimulator,
        motion_controller: Option<Box<dyn MotionController>>,
        orientation_controller: Option<Box<dyn OrientationController>>,
    ) -> Self {
        let window = Arc::new(window);
        Self {
            window: Arc::clone(&window),
            user_interface: RwLock::new(UserInterface::new(window)),
            ecs_world: RwLock::new(ECSWorld::new()),
            scene: RwLock::new(scene),
            renderer: RwLock::new(renderer),
            simulator: RwLock::new(simulator),
            motion_controller: motion_controller.map(Mutex::new),
            orientation_controller: orientation_controller.map(Mutex::new),
        }
    }

    /// Returns a reference to the [`Window`].
    pub fn window(&self) -> &Window {
        self.window.as_ref()
    }

    /// Returns a reference to the [`UserInterface`], guarded
    /// by a [`RwLock`].
    pub fn user_interface(&self) -> &RwLock<UserInterface> {
        &self.user_interface
    }

    /// Returns a reference to the ECS [`World`](impact_ecs::world::World), guarded
    /// by a [`RwLock`].
    pub fn ecs_world(&self) -> &RwLock<ECSWorld> {
        &self.ecs_world
    }

    /// Returns a reference to the [`Scene`], guarded
    /// by a [`RwLock`].
    pub fn scene(&self) -> &RwLock<Scene> {
        &self.scene
    }

    /// Returns a reference to the [`RenderingSystem`], guarded
    /// by a [`RwLock`].
    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Returns a reference to the [`PhysicsSimulator`], guarded
    /// by a [`RwLock`].
    pub fn simulator(&self) -> &RwLock<PhysicsSimulator> {
        &self.simulator
    }

    pub fn create_entities<'a, E>(
        &self,
        components: impl TryInto<ArchetypeCompByteView<'a>, Error = E>,
    ) -> Result<Vec<Entity>>
    where
        E: Into<anyhow::Error>,
    {
        let mut manager =
            ComponentManager::with_initial_components(components.try_into().map_err(E::into)?)?;

        setup!(
            {
                let scene = self.scene().read().unwrap();
                let mut scene_graph = scene.scene_graph().write().unwrap();
                let root_node_id = scene_graph.root_node_id();
            },
            manager,
            |camera: &CameraComp,
             position: &PositionComp,
             orientation: &OrientationComp|
             -> SceneGraphNodeComp::<CameraNodeID> {
                let camera_to_world_transform =
                    sc::model_to_world_transform_from_position_and_orientation(
                        position.0.cast(),
                        orientation.0.cast(),
                    );

                let node_id = scene_graph.create_camera_node(
                    root_node_id,
                    camera_to_world_transform,
                    camera.id,
                );

                scene.set_active_camera(Some((camera.id, node_id)));

                SceneGraphNodeComp::new(node_id)
            }
        );

        setup!(
            {
                let scene = self.scene().read().unwrap();
                let mesh_repository = scene.mesh_repository().read().unwrap();
                let mut instance_transform_pool =
                    scene.model_instance_transform_pool().write().unwrap();
                let mut scene_graph = scene.scene_graph().write().unwrap();
                let root_node_id = scene_graph.root_node_id();
            },
            manager,
            |mesh: &MeshComp,
             material: &MaterialComp,
             position: &PositionComp,
             orientation: &OrientationComp|
             -> SceneGraphNodeComp::<ModelInstanceNodeID> {
                let model_id = ModelID::for_mesh_and_material(mesh.id, material.id);
                instance_transform_pool.increment_user_count(model_id);

                let model_to_world_transform =
                    sc::model_to_world_transform_from_position_and_orientation(
                        position.0.cast(),
                        orientation.0.cast(),
                    );

                // Panic on errors since returning an error could leave us
                // in an inconsistent state
                let bounding_sphere = mesh_repository
                    .get_mesh(mesh.id)
                    .expect("Tried to create renderable entity with mesh not present in mesh repository")
                    .bounding_sphere()
                    .expect("Tried to create renderable entity with empty mesh");

                SceneGraphNodeComp::new(scene_graph.create_model_instance_node(
                    root_node_id,
                    model_to_world_transform,
                    model_id,
                    bounding_sphere,
                ))
            }
        );

        self.ecs_world.write().unwrap().create_entities(&manager)
    }

    pub fn remove_entity(&self, entity: &Entity) -> Result<()> {
        let mut ecs_world = self.ecs_world.write().unwrap();

        let entry = ecs_world.entity(entity);

        if let Some(node) = entry.get_component::<SceneGraphNodeComp<CameraNodeID>>() {
            let node_id = node.access().id;

            let scene = self.scene().read().unwrap();

            scene
                .scene_graph()
                .write()
                .unwrap()
                .remove_camera_node(node_id);

            if let Some(active_camera_node_id) = scene.get_active_camera_node_id() {
                if active_camera_node_id == node_id {
                    scene.set_active_camera(None);
                }
            }
        }

        if let Some(node) = entry.get_component::<SceneGraphNodeComp<ModelInstanceNodeID>>() {
            let scene = self.scene().read().unwrap();
            let model_id = scene
                .scene_graph()
                .write()
                .unwrap()
                .remove_model_instance_node(node.access().id);
            scene
                .model_instance_transform_pool()
                .write()
                .unwrap()
                .decrement_user_count(model_id);
        }

        drop(entry);
        ecs_world.remove_entity(entity)
    }

    /// Sets a new size for the rendering surface and updates
    /// the aspect ratio of all cameras.
    pub fn resize_rendering_surface(&self, new_size: (u32, u32)) {
        self.renderer().write().unwrap().resize_surface(new_size);

        self.scene()
            .read()
            .unwrap()
            .camera_repository()
            .write()
            .unwrap()
            .set_aspect_ratios(window::calculate_aspect_ratio(new_size.0, new_size.1));

        self.renderer()
            .read()
            .unwrap()
            .render_resource_manager()
            .write()
            .unwrap()
            .declare_desynchronized();
    }

    pub fn toggle_interaction_mode(&self) {
        let mut user_interface = self.user_interface().write().unwrap();
        if user_interface.control_mode_active() {
            self.stop_motion_controller();
            user_interface.activate_cursor_mode();
        } else {
            user_interface.activate_control_mode();
        }
    }

    pub fn control_mode_active(&self) -> bool {
        self.user_interface().read().unwrap().control_mode_active()
    }

    /// Updates the motion controller with the given motion.
    pub fn update_motion_controller(&self, state: MotionState, direction: MotionDirection) {
        if let Some(motion_controller) = &self.motion_controller {
            log::debug!(
                "Updating motion controller to state {:?} and direction {:?}",
                state,
                direction
            );

            let mut motion_controller = motion_controller.lock().unwrap();

            let result = motion_controller.update_motion(state, direction);

            if result.motion_changed() {
                control::set_velocities_of_controlled_entities(
                    &self.ecs_world().read().unwrap(),
                    motion_controller.as_ref(),
                );
            }
        }
    }

    fn stop_motion_controller(&self) {
        if let Some(motion_controller) = &self.motion_controller {
            let mut motion_controller = motion_controller.lock().unwrap();

            let result = motion_controller.stop();

            if result.motion_changed() {
                control::set_velocities_of_controlled_entities(
                    &self.ecs_world().read().unwrap(),
                    motion_controller.as_ref(),
                );
            }
        }
    }

    /// Updates the orientation controller with the given mouse
    /// displacement.
    pub fn update_orientation_controller(&self, mouse_displacement: (f64, f64)) {
        if let Some(orientation_controller) = &self.orientation_controller {
            log::info!(
                "Updating orientation controller by mouse delta ({}, {})",
                mouse_displacement.0,
                mouse_displacement.1
            );

            let mut orientation_controller = orientation_controller.lock().unwrap();

            orientation_controller.update_orientation_change(self.window(), mouse_displacement);

            let ecs_world = self.ecs_world().read().unwrap();
            control::update_orientations_of_controlled_entities(
                &ecs_world,
                orientation_controller.as_ref(),
            );
            if let Some(motion_controller) = &self.motion_controller {
                control::set_velocities_of_controlled_entities(
                    &ecs_world,
                    motion_controller.lock().unwrap().as_ref(),
                );
            }
        }
    }

    /// Creates a new task scheduler with the given number of
    /// workers and registers all tasks in it.
    ///
    /// # Errors
    /// Returns an error the registration of any of the tasks
    /// failed.
    pub fn create_task_scheduler(
        self,
        n_workers: NonZeroUsize,
    ) -> Result<(Arc<Self>, WorldTaskScheduler)> {
        let world = Arc::new(self);
        let mut task_scheduler = WorldTaskScheduler::new(n_workers, Arc::clone(&world));

        Self::register_all_tasks(&mut task_scheduler)?;

        Ok((world, task_scheduler))
    }

    /// Identifies errors that need special handling in the given
    /// set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        control_flow: &mut ControlFlow<'_>,
    ) {
        self.simulator
            .read()
            .unwrap()
            .handle_task_errors(task_errors, control_flow);

        self.scene
            .read()
            .unwrap()
            .handle_task_errors(task_errors, control_flow);

        self.renderer
            .read()
            .unwrap()
            .handle_task_errors(task_errors, control_flow);
    }

    /// Registers all tasks in the given task scheduler.
    fn register_all_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        Scene::register_tasks(task_scheduler)?;
        RenderingSystem::register_tasks(task_scheduler)?;
        PhysicsSimulator::register_tasks(task_scheduler)?;
        task_scheduler.complete_task_registration()
    }
}
