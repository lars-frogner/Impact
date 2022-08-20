use crate::{
    define_task,
    rendering::RenderingTag,
    scheduling::Task,
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    [pub] SyncVisibleModelInstances,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing visible model instances"; world.sync_visible_model_instances())
    }
);

impl World {
    pub fn register_world_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncVisibleModelInstances)
    }

    pub fn handle_world_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        control_flow: &mut ControlFlow<'_>,
    ) {
        if let Err(error) = task_errors.take_result_of(SyncVisibleModelInstances.id()) {
            log::error!("{:?}", error);
            control_flow.exit();
        }
    }
}
