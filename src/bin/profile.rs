use impact::voxel::{
    generation::{UniformBoxVoxelGenerator, UniformSphereVoxelGenerator},
    ChunkedVoxelObject, VoxelType,
};
use std::{
    fmt::Display,
    hint::black_box,
    time::{Duration, Instant},
};

const TARGETS: [&str; 2] = [
    "chunked_voxel_object_construction",
    "chunked_voxel_object_initialize_adjacencies",
];

fn main() {
    let target = if let Some(target) = std::env::args().nth(1) {
        target
    } else {
        exit_with_error("Usage: profile <target> <duration in seconds>", true);
    };
    let duration = if let Some(duration) = std::env::args().nth(2) {
        match duration.parse() {
            Ok(duration) => Duration::from_secs(duration),
            Err(_) => exit_with_error("Duration must be a positive integer", false),
        }
    } else {
        exit_with_error("Usage: profile <target> <duration in seconds>", false);
    };

    match target.as_str() {
        "chunked_voxel_object_construction" => profile_chunked_voxel_object_construction(duration),
        "chunked_voxel_object_initialize_adjacencies" => {
            profile_chunked_voxel_object_initialize_adjacencies(duration)
        }
        _ => {
            exit_with_error(format!("Unknown target: {}", target), true);
        }
    }
}

fn exit_with_error(message: impl Display, list_targets: bool) -> ! {
    eprintln!("{}", message);
    if list_targets {
        eprintln!("Available targets:");
        for target in TARGETS {
            eprintln!("- {}", target);
        }
    }
    std::process::exit(1)
}

fn profile_chunked_voxel_object_construction(duration: Duration) {
    let generator = UniformBoxVoxelGenerator::new(VoxelType::Default, 0.25, 200, 200, 200);
    let start = Instant::now();
    while start.elapsed() < duration {
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        black_box(object);
    }
}

fn profile_chunked_voxel_object_initialize_adjacencies(duration: Duration) {
    let generator = UniformSphereVoxelGenerator::new(VoxelType::Default, 0.25, 200);
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    let start = Instant::now();
    while start.elapsed() < duration {
        let mut object = object.clone();
        object.initialize_adjacencies();
        black_box(object);
    }
}
