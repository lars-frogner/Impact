//! Utilities for multithreading with `rayon`.

use rayon::{ThreadPool, ThreadPoolBuilder};
use std::num::NonZeroUsize;

#[derive(Debug)]
pub struct RayonThreadPool {
    pool: ThreadPool,
    num_threads: NonZeroUsize,
}

impl RayonThreadPool {
    pub fn new(num_threads: NonZeroUsize) -> Self {
        let pool = ThreadPoolBuilder::new()
            .num_threads(num_threads.get())
            .build()
            .unwrap();

        Self { pool, num_threads }
    }

    pub fn pool(&self) -> &ThreadPool {
        &self.pool
    }

    pub fn num_threads(&self) -> NonZeroUsize {
        self.num_threads
    }
}
