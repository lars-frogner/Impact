//! Sorting benchmarks.

use impact_math::{random::Rng, sorting};
use impact_profiling::benchmark::Benchmarker;

pub fn radix_sort_u64(benchmarker: impl Benchmarker) {
    radix_sort_n_u64(benchmarker, 1000000);
}

pub fn radix_sort_by_u64_keys(benchmarker: impl Benchmarker) {
    radix_sort_by_n_u64_keys(benchmarker, 1000000);
}

pub fn std_sort_u64(benchmarker: impl Benchmarker) {
    std_sort_n_u64(benchmarker, 1000000);
}

pub fn radix_sort_n_u64(benchmarker: impl Benchmarker, n: usize) {
    let mut rng = Rng::with_seed(0);
    let mut values = vec![0_u64; n];
    rng.fill_byte_slice(bytemuck::cast_slice_mut(values.as_mut_slice()));

    let mut values_tmp = values.clone();

    benchmarker.benchmark(&mut || {
        values_tmp.copy_from_slice(&values);
        sorting::radix_sort_u64(&mut values_tmp);
    });
}

pub fn radix_sort_by_n_u64_keys(benchmarker: impl Benchmarker, n: usize) {
    let mut values = vec![0_u64; n];
    let mut rng = Rng::with_seed(0);
    rng.fill_byte_slice(bytemuck::cast_slice_mut(values.as_mut_slice()));

    let mut sorted_indices = vec![0; n];

    benchmarker.benchmark(&mut || {
        sorting::radix_sort_by_u64_keys(&values, |src_idx, dest_idx| {
            sorted_indices[dest_idx] = src_idx;
        });
    });
}

pub fn std_sort_n_u64(benchmarker: impl Benchmarker, n: usize) {
    let mut rng = Rng::with_seed(0);
    let mut values = vec![0_u64; n];
    rng.fill_byte_slice(bytemuck::cast_slice_mut(values.as_mut_slice()));

    let mut values_tmp = values.clone();

    benchmarker.benchmark(&mut || {
        values_tmp.copy_from_slice(&values);
        values_tmp.sort_unstable();
    });
}
