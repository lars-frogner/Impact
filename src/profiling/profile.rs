//! Utilities for initiating profiling.

use crate::profiling::{BasicProfiler, Delayer, Profiler, benchmarks};
use std::time::{Duration, Instant};

macro_rules! define_target_enum {
(
    $(
        $module:ident => {
            $($func:ident),* $(,)?
        }
    ),* $(,)?
) => {
    ::paste::paste! {
        #[allow(clippy::enum_variant_names)]
        #[cfg_attr(feature = "cli", derive(::clap::ValueEnum))]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Target {
            $(
                $( [<$module:camel $func:camel>] ),*
            ),*
        }

        impl Target {
            fn execute(&self, profiler: impl Profiler) {
                match self {
                    $(
                        $( Self::[<$module:camel $func:camel>] => benchmarks::$module::$func(profiler), )*
                    )*
                }
            }
        }
    }};
}

define_target_enum! {
    chunked_voxel_object => {
        construction,
        update_internal_adjacencies_for_all_chunks,
        update_connected_regions_for_all_chunks,
        update_all_chunk_boundary_adjacencies,
        resolve_connected_regions_between_all_chunks,
        compute_all_derived_state,
        initialize_inertial_properties,
        create_mesh,
        modify_voxels_within_sphere,
        split_off_disconnected_region,
        split_off_disconnected_region_with_inertial_property_transfer,
        update_mesh,
    },
    model => {
        add_feature_to_dynamic_instance_buffer_from_storage,
        add_feature_to_dynamic_instance_buffer_from_storage_repeatedly,
    },
    constraint => {
        prepare_contacts,
        solve_contact_velocities,
        correct_contact_configurations,
    },
}

pub fn profile(target: Target, duration: f64, delay: f64) {
    let start = Instant::now();

    let delayer = Delayer::new(start, delay);
    let duration = Duration::from_secs_f64(duration);

    let profiler = BasicProfiler::new(duration, delayer);

    target.execute(profiler);
}
