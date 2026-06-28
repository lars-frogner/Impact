//! Command buffering and execution.

use crate::App;
use impact::{
    command::queue::CommandQueue,
    impact_id::EntityID,
    impact_voxel::interaction::fracturing::{
        FracturePointGenerator, RandomizedGridFracturePointGenerator,
    },
};
use roc_integration::roc;

pub static APP_COMMANDS: AppCommandQueue = AppCommandQueue::new();

pub type AppCommandQueue = CommandQueue<AppCommand>;

#[roc(parents = "Command")]
#[derive(Clone, Debug, PartialEq)]
pub enum AppCommand {
    FractureVoxelObject {
        entity_id: EntityID,
        points_per_dim: u64,
    },
}

impl App {
    pub(crate) fn execute_app_commands(&mut self) {
        APP_COMMANDS.execute_commands(|command| match command {
            AppCommand::FractureVoxelObject {
                entity_id,
                points_per_dim,
            } => {
                log::debug!("Fracturing voxel object entity {entity_id}");
                let fracture_point_generator = FracturePointGenerator::RandomizedGrid(
                    RandomizedGridFracturePointGenerator::new(points_per_dim as usize),
                );
                if let Err(error) =
                    self.engine()
                        .fracture_voxel_object(entity_id, &fracture_point_generator, 0)
                {
                    log::error!("Failed to fracture voxel object: {error}");
                }
            }
        });
    }
}
