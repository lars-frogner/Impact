#[cfg(not(feature = "profiling"))]
compile_error!("This binary requires the 'profiling' feature to be enabled.");

#[cfg(feature = "profiling")]
mod run {
    use clap::Parser;
    use impact::profiling::{BasicProfiler, Delayer};
    use std::time::{Duration, Instant};

    #[derive(Parser, Debug)]
    #[command(about = "Run a profiling target", long_about = None)]
    struct Args {
        /// Profiling target to run
        #[arg(short, long, value_enum)]
        target: Target,

        /// Number of seconds to run the target for (it will always be run at least
        /// once)
        #[arg(short, long, default_value_t = 0.0)]
        duration: f64,

        /// Minimum number of seconds from the program is started until the target
        /// is run
        #[arg(long, default_value_t = 0.0)]
        delay: f64,
    }

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
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, ::clap::ValueEnum)]
            enum Target {
                $(
                    $( [<$module:camel $func:camel>] ),*
                ),*
            }

            impl Target {
                fn execute(&self, profiler: impl ::impact::profiling::Profiler) {
                    match self {
                        $(
                            $( Self::[<$module:camel $func:camel>] => impact::profiling::benchmarks::$module::$func(profiler), )*
                        )*
                    }
                }
            }
        }
    };
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

    pub fn run() {
        let program_start = Instant::now();

        let args = Args::parse();

        let delayer = Delayer::new(program_start, args.delay);
        let duration = Duration::from_secs_f64(args.duration);

        let profiler = BasicProfiler::new(duration, delayer);

        args.target.execute(profiler);
    }
}

fn main() {
    #[cfg(feature = "profiling")]
    run::run();
}
