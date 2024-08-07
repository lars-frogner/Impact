#![no_main]

use impact::voxel::{generation::fuzzing::ArbitraryVoxelGenerator, ChunkedVoxelObject};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: ArbitraryVoxelGenerator| {
    let object = match generator {
        ArbitraryVoxelGenerator::UniformBox(generator) => ChunkedVoxelObject::generate(&generator),
        ArbitraryVoxelGenerator::UniformSphere(generator) => {
            ChunkedVoxelObject::generate(&generator)
        }
        ArbitraryVoxelGenerator::GradientNoise(generator) => {
            ChunkedVoxelObject::generate(&generator)
        }
    };
    if let Some(mut object) = object {
        object.initialize_adjacencies();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
        object.validate_superchunk_obscuredness();
    }
});
