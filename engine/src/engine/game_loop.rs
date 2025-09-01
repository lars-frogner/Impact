//! Executing the engine's game loop.

use super::Engine;
use crate::{
    instrumentation,
    lock_order::OrderedRwLock,
    runtime::tasks::RuntimeTaskScheduler,
    tasks::{PhysicsTag, RenderingTag, UserInterfaceTag},
};
use anyhow::Result;
use impact_scheduling::define_execution_tag_set;
use std::time::Instant;

define_execution_tag_set!(ALL_SYSTEMS, [PhysicsTag, RenderingTag, UserInterfaceTag]);

impl Engine {
    pub(crate) fn perform_game_loop_iteration(
        &self,
        task_scheduler: &RuntimeTaskScheduler,
    ) -> Result<()> {
        let game_loop_controller = self.game_loop_controller.oread();

        if game_loop_controller.reached_max_iterations() {
            impact_log::info!("Reached max iterations, requesting shutdown");
            self.request_shutdown();
            return Ok(());
        }

        if !game_loop_controller.should_perform_iteration() {
            return Ok(());
        }

        drop(game_loop_controller);

        let iter_start_time = Instant::now();

        impact_profiling::instrumentation::frame_mark();

        let execution_result = task_scheduler.execute_and_wait(&ALL_SYSTEMS);

        if let Err(mut task_errors) = execution_result {
            self.handle_task_errors(&mut task_errors);

            // Pass any unhandled errors to caller
            if task_errors.n_errors() > 0 {
                return Err(task_errors.into());
            }
        }

        let mut game_loop_controller = self.game_loop_controller.owrite();

        let iter_end_time = game_loop_controller.wait_for_target_frame_duration(iter_start_time);

        let frame_duration = iter_end_time - iter_start_time;
        game_loop_controller.add_frame_duration(frame_duration);

        let smooth_frame_duration = game_loop_controller.compute_smooth_frame_duration();

        self.gather_metrics_after_completed_frame(smooth_frame_duration);
        self.update_simulation_time_step_duration(smooth_frame_duration);

        impact_log::info!(
            "Completed game loop iteration after {:.1} ms (~{} FPS, {:.1} s elapsed)",
            frame_duration.as_secs_f64() * 1e3,
            instrumentation::frame_duration_to_fps(smooth_frame_duration),
            game_loop_controller.elapsed_time().as_secs_f64()
        );

        game_loop_controller.increment_iteration();

        Ok(())
    }
}
