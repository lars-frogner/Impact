use clap::{Parser, ValueEnum};
use impact::{
    geometry::Sphere,
    voxel::{
        chunks::ChunkedVoxelObject,
        generation::{
            BoxSDFGenerator, SDFVoxelGenerator, SameVoxelTypeGenerator, SphereSDFGenerator,
        },
        mesh::ChunkedVoxelObjectMesh,
        voxel_types::VoxelType,
    },
};
use nalgebra::{vector, UnitVector3};
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
    ChunkedVoxelObjectInitializeAdjacencies,
    ChunkedVoxelObjectCreateMesh,
    ChunkedVoxelObjectModifyVoxelsWithinSphere,
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
        Target::ChunkedVoxelObjectInitializeAdjacencies => {
            profile_chunked_voxel_object_initialize_adjacencies(duration, delayer);
        }
        Target::ChunkedVoxelObjectCreateMesh => {
            profile_chunked_voxel_object_create_mesh(duration, delayer);
        }
        Target::ChunkedVoxelObjectModifyVoxelsWithinSphere => {
            profile_chunked_voxel_object_modify_voxels_within_sphere(duration, delayer);
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
        &mut || ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap(),
        duration,
        delayer,
    );
}

fn profile_chunked_voxel_object_initialize_adjacencies(duration: Duration, delayer: Delayer) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
    profile(
        &mut || {
            let mut object = object.clone();
            object.initialize_adjacencies();
            object
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
            mesh.sync_with_voxel_object(&mut object);
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
