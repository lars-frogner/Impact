use clap::{Parser, ValueEnum};
use impact::{
    geometry::Sphere,
    scene::RenderResourcesDesynchronized,
    voxel::{
        chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyManager},
        generation::{
            BoxSDFGenerator, SDFUnion, SDFVoxelGenerator, SameVoxelTypeGenerator,
            SphereSDFGenerator,
        },
        mesh::ChunkedVoxelObjectMesh,
        voxel_types::VoxelType,
    },
};
use nalgebra::{UnitVector3, vector};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

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

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Target {
    ChunkedVoxelObjectConstruction,
    ChunkedVoxelObjectUpdateInternalAdjacenciesForAllChunks,
    ChunkedVoxelObjectUpdateConnectedRegionsForAllChunks,
    ChunkedVoxelObjectUpdateAllChunkBoundaryAdjacencies,
    ChunkedVoxelObjectResolveConnectedRegionsBetweenAllChunks,
    ChunkedVoxelObjectComputeAllDerivedState,
    ChunkedVoxelObjectInitializeInertialProperties,
    ChunkedVoxelObjectCreateMesh,
    ChunkedVoxelObjectModifyVoxelsWithinSphere,
    ChunkedVoxelObjectSplitOffDisconnectedRegion,
    ChunkedVoxelObjectSplitOffDisconnectedRegionWithInertialPropertyTransfer,
    ChunkedVoxelObjectUpdateMesh,
}

#[derive(Debug)]
struct Delayer {
    program_start: Instant,
    delay: Duration,
}

impl Delayer {
    fn new(program_start: Instant, delay_seconds: f64) -> Self {
        Self {
            program_start,
            delay: Duration::from_secs_f64(delay_seconds),
        }
    }

    fn wait(self) {
        let remaining = self.delay.saturating_sub(self.program_start.elapsed());
        if remaining > Duration::ZERO {
            std::thread::sleep(remaining);
        }
    }
}

fn main() {
    let program_start = Instant::now();

    let args = Args::parse();

    let delayer = Delayer::new(program_start, args.delay);

    let duration = Duration::from_secs_f64(args.duration);

    match args.target {
        Target::ChunkedVoxelObjectConstruction => {
            profile_chunked_voxel_object_construction(duration, delayer);
        }
        Target::ChunkedVoxelObjectUpdateInternalAdjacenciesForAllChunks => {
            profile_chunked_voxel_object_update_internal_adjacencies_for_all_chunks(
                duration, delayer,
            );
        }
        Target::ChunkedVoxelObjectUpdateConnectedRegionsForAllChunks => {
            profile_chunked_voxel_object_update_connected_regions_for_all_chunks(duration, delayer);
        }
        Target::ChunkedVoxelObjectUpdateAllChunkBoundaryAdjacencies => {
            profile_chunked_voxel_object_update_all_chunk_boundary_adjacencies(duration, delayer);
        }
        Target::ChunkedVoxelObjectResolveConnectedRegionsBetweenAllChunks => {
            profile_chunked_voxel_object_resolve_connected_regions_between_all_chunks(
                duration, delayer,
            );
        }
        Target::ChunkedVoxelObjectComputeAllDerivedState => {
            profile_chunked_voxel_object_compute_all_derived_state(duration, delayer);
        }
        Target::ChunkedVoxelObjectInitializeInertialProperties => {
            profile_chunked_voxel_object_initialize_inertial_properties(duration, delayer);
        }
        Target::ChunkedVoxelObjectCreateMesh => {
            profile_chunked_voxel_object_create_mesh(duration, delayer);
        }
        Target::ChunkedVoxelObjectModifyVoxelsWithinSphere => {
            profile_chunked_voxel_object_modify_voxels_within_sphere(duration, delayer);
        }
        Target::ChunkedVoxelObjectSplitOffDisconnectedRegion => {
            profile_chunked_voxel_object_split_off_disconnected_region(duration, delayer);
        }
        Target::ChunkedVoxelObjectSplitOffDisconnectedRegionWithInertialPropertyTransfer => {
            profile_chunked_voxel_object_split_off_disconnected_region_with_inertial_property_transfer(duration, delayer);
        }
        Target::ChunkedVoxelObjectUpdateMesh => {
            profile_chunked_voxel_object_update_mesh(duration, delayer);
        }
    }
}

fn profile_chunked_voxel_object_construction(duration: Duration, delayer: Delayer) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        BoxSDFGenerator::new([200.0; 3]),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    profile(
        &mut || ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap(),
        duration,
        delayer,
    );
}

fn profile_chunked_voxel_object_update_internal_adjacencies_for_all_chunks(
    duration: Duration,
    delayer: Delayer,
) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    profile(
        &mut || {
            object.update_internal_adjacencies_for_all_chunks();
        },
        duration,
        delayer,
    );
    black_box(object);
}

fn profile_chunked_voxel_object_update_connected_regions_for_all_chunks(
    duration: Duration,
    delayer: Delayer,
) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    profile(
        &mut || {
            object.update_local_connected_regions_for_all_chunks();
        },
        duration,
        delayer,
    );
    black_box(object);
}

fn profile_chunked_voxel_object_update_all_chunk_boundary_adjacencies(
    duration: Duration,
    delayer: Delayer,
) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    profile(
        &mut || {
            object.update_all_chunk_boundary_adjacencies();
        },
        duration,
        delayer,
    );
    black_box(object);
}

fn profile_chunked_voxel_object_resolve_connected_regions_between_all_chunks(
    duration: Duration,
    delayer: Delayer,
) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    object.update_all_chunk_boundary_adjacencies();
    profile(
        &mut || {
            object.resolve_connected_regions_between_all_chunks();
        },
        duration,
        delayer,
    );
    black_box(object);
}

fn profile_chunked_voxel_object_compute_all_derived_state(duration: Duration, delayer: Delayer) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    profile(
        &mut || {
            object.compute_all_derived_state();
        },
        duration,
        delayer,
    );
    black_box(object);
}

fn profile_chunked_voxel_object_initialize_inertial_properties(
    duration: Duration,
    delayer: Delayer,
) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let voxel_type_densities = [1.0; 256];
    let object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    profile(
        &mut || {
            VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities)
        },
        duration,
        delayer,
    );
}

fn profile_chunked_voxel_object_create_mesh(duration: Duration, delayer: Delayer) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    profile(
        &mut || ChunkedVoxelObjectMesh::create(&object),
        duration,
        delayer,
    );
}

fn profile_chunked_voxel_object_modify_voxels_within_sphere(duration: Duration, delayer: Delayer) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(object_radius),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
    let sphere = Sphere::new(
        object.compute_aabb::<f64>().center()
            - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        sphere_radius,
    );
    profile(
        &mut || {
            object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
                black_box((indices, position, voxel));
            });
        },
        duration,
        delayer,
    );
}

fn profile_chunked_voxel_object_split_off_disconnected_region(
    duration: Duration,
    delayer: Delayer,
) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SDFUnion::new(
            SphereSDFGenerator::new(50.0),
            SphereSDFGenerator::new(50.0),
            [120.0, 0.0, 0.0],
            1.0,
        ),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    profile(
        &mut || object.clone().split_off_any_disconnected_region().unwrap(),
        duration,
        delayer,
    );
}

fn profile_chunked_voxel_object_split_off_disconnected_region_with_inertial_property_transfer(
    duration: Duration,
    delayer: Delayer,
) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SDFUnion::new(
            SphereSDFGenerator::new(50.0),
            SphereSDFGenerator::new(50.0),
            [120.0, 0.0, 0.0],
            1.0,
        ),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let voxel_type_densities = [1.0; 256];
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    let inertial_property_manager =
        VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities);
    profile(
        &mut || {
            let mut inertial_property_manager = inertial_property_manager.clone();
            let mut disconnected_inertial_property_manager =
                VoxelObjectInertialPropertyManager::zeroed();
            let mut inertial_property_transferrer = inertial_property_manager.begin_transfer_to(
                &mut disconnected_inertial_property_manager,
                object.voxel_extent(),
                &voxel_type_densities,
            );
            let disconnected_object = object
                .clone()
                .split_off_any_disconnected_region_with_property_transferrer(
                    &mut inertial_property_transferrer,
                )
                .unwrap();
            (
                disconnected_object,
                inertial_property_manager,
                disconnected_inertial_property_manager,
            )
        },
        duration,
        delayer,
    );
}

fn profile_chunked_voxel_object_update_mesh(duration: Duration, delayer: Delayer) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(object_radius),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
    let mut mesh = ChunkedVoxelObjectMesh::create(&object);

    let sphere = Sphere::new(
        object.compute_aabb::<f64>().center()
            - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        sphere_radius,
    );

    profile(
        &mut || {
            object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
                black_box((indices, position, voxel));
            });
            let mut desynchronized = RenderResourcesDesynchronized::No;
            mesh.sync_with_voxel_object(&mut object, &mut desynchronized);
            black_box((&object, &mesh));
        },
        duration,
        delayer,
    );
}

fn profile<T>(f: &mut impl FnMut() -> T, duration: Duration, delayer: Delayer) {
    delayer.wait();
    let start = Instant::now();
    loop {
        black_box(f());

        if start.elapsed() > duration {
            break;
        }
    }
}
