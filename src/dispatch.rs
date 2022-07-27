use crate::thread::{ThreadCommunicator, ThreadPool};
use anyhow::{anyhow, bail, Result};
use const_fnv1a_hash;
use petgraph::{
    algo::{self, DfsSpace},
    graphmap::DiGraphMap,
};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[cfg(test)]
use crate::thread::WorkerID;

pub type JobID = u64;

pub trait Job<S>: Sync + Send {
    fn id(&self) -> JobID;

    fn depends_on(&self) -> &[JobID];

    fn run(&self, world_state: &S) -> Result<()>;

    #[cfg(test)]
    fn run_with_worker(&self, _worker_id: WorkerID, world_state: &S) -> Result<()> {
        self.run(world_state)
    }
}

type DispatcherThreadPool<S> = ThreadPool<JobMessage<S>>;
type JobPool<S> = HashMap<JobID, Arc<dyn Job<S>>>;
type JobMessage<S> = (Arc<JobExecutionState<S>>, usize);

pub struct Dispatcher<S> {
    n_workers: NonZeroUsize,
    jobs: JobPool<S>,
    dependency_graph: JobDependencyGraph<S>,
    executor: Option<JobExecutor<S>>,
    world_state: Arc<S>,
}

struct JobDependencyGraph<S> {
    graph: DiGraphMap<JobID, ()>,
    space: DfsSpace<JobID, HashSet<JobID>>,
    independent_jobs: HashSet<JobID>,
    _phantom: PhantomData<S>,
}

struct JobExecutor<S> {
    state: Arc<JobExecutionState<S>>,
    thread_pool: DispatcherThreadPool<S>,
}

struct JobExecutionState<S> {
    job_ordering: JobOrdering<S>,
    world_state: Arc<S>,
}

struct JobOrdering<S> {
    jobs: Vec<OrderedJob<S>>,
    n_dependencyless_jobs: usize,
}

struct OrderedJob<S> {
    job: Arc<dyn Job<S>>,
    n_dependencies: usize,
    indices_of_dependent_jobs: Vec<usize>,
    completed_dependency_count: AtomicUsize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum JobReady {
    Yes,
    No,
}

pub const fn hash_job_name_to_id(name: &str) -> JobID {
    const_fnv1a_hash::fnv1a_hash_str_64(name)
}

impl<S> Dispatcher<S>
where
    S: Sync + Send + 'static,
{
    pub fn new(n_workers: NonZeroUsize, world_state: Arc<S>) -> Self {
        Self {
            n_workers,
            jobs: HashMap::new(),
            dependency_graph: JobDependencyGraph::new(),
            executor: None,
            world_state,
        }
    }

    pub fn world_state(&self) -> &S {
        self.world_state.as_ref()
    }

    pub fn has_job(&self, job_id: JobID) -> bool {
        self.jobs.contains_key(&job_id)
    }

    pub fn execute(&self) {
        self.executor
            .as_ref()
            .expect("Called `execute` before completing job registration")
            .execute();
    }

    pub fn execute_on_main_thread(&self) {
        self.executor
            .as_ref()
            .expect("Called `execute` before completing job registration")
            .execute_on_main_thread();
    }

    pub fn register_job(&mut self, job: impl Job<S> + 'static) -> Result<()> {
        let job_id = job.id();
        if self.jobs.contains_key(&job_id) {
            bail!("Job with ID {} already exists", job_id);
        }

        self.dependency_graph.add_job(&job)?;

        self.jobs.insert(job.id(), Arc::new(job));

        // Changing the jobs invalidates the executor
        self.executor = None;

        Ok(())
    }

    pub fn complete_job_registration(&mut self) -> Result<()> {
        self.executor = Some(JobExecutor::new(
            self.n_workers,
            &self.jobs,
            &mut self.dependency_graph,
            Arc::clone(&self.world_state),
        )?);
        Ok(())
    }

    #[allow(dead_code)]
    fn executor(&self) -> &JobExecutor<S> {
        self.executor.as_ref().unwrap()
    }
}

impl<S> JobDependencyGraph<S> {
    fn new() -> Self {
        let graph = DiGraphMap::new();
        let space = DfsSpace::new(&graph);
        let independent_jobs = HashSet::new();
        Self {
            graph,
            space,
            independent_jobs,
            _phantom: PhantomData,
        }
    }

    fn add_job(&mut self, job: &impl Job<S>) -> Result<()> {
        let job_id = job.id();
        self.graph.add_node(job_id);

        let dependence_job_ids = job.depends_on();

        if dependence_job_ids.is_empty() {
            self.independent_jobs.insert(job_id);
        }

        for &dependence_job_id in dependence_job_ids {
            // Add edge directed from dependence to dependent.
            // A node for the dependence job is added if it
            // doesn't exist.
            let existing_edge = self.graph.add_edge(dependence_job_id, job_id, ());

            if existing_edge.is_some() {
                bail!(
                    "Job with ID {} depends on same job (ID {}) multiple times",
                    job_id,
                    dependence_job_id
                );
            }
        }
        Ok(())
    }

    fn obtain_ordered_job_ids(&mut self) -> Result<Vec<JobID>> {
        let n_jobs = self.graph.node_count();
        let mut sorted_ids = Vec::with_capacity(n_jobs);

        // Make sure all jobs without dependencies come first
        sorted_ids.extend(self.independent_jobs.iter());

        if n_jobs > self.independent_jobs.len() {
            // Get job IDs sorted to topological order, meaning an order
            // where each job comes after all its dependencies
            let topologically_sorted_ids = algo::toposort(&self.graph, Some(&mut self.space))
                .map_err(|_| anyhow!("Found circular job dependencies"))?;

            // Add all jobs with dependencies in topological order
            sorted_ids.extend(
                topologically_sorted_ids
                    .into_iter()
                    .filter(|job_id| !self.independent_jobs.contains(job_id)),
            );
        }

        assert_eq!(sorted_ids.len(), n_jobs);

        Ok(sorted_ids)
    }

    fn find_dependent_job_ids(&self, job_id: JobID) -> impl Iterator<Item = JobID> + '_ {
        // Find outgoing edges, i.e. to jobs depending on this one
        self.graph
            .edges(job_id)
            .map(|(_job_id, dependent_job_id, _)| dependent_job_id)
    }
}

/// Job execution message contains the index of the
/// job to execute.
///
/// Each job holds the indices of its dependent jobs.
///
/// Find the number of jobs with no dependencies. Send
/// an execution message for each of these.
/// For each job there is a counter specifying the number
/// of jobs it depends on that have yet to complete.
/// When a job is completed, go to each dependent job
/// and decrement their counter. If no counters reached
/// zero, go to sleep. If one counter reached zero,
/// start on that job. For each additional counter
/// that reached zero, send an execution message with
/// the index of that job.
impl<S> JobExecutor<S>
where
    S: Sync + Send + 'static,
{
    fn new(
        n_workers: NonZeroUsize,
        job_pool: &JobPool<S>,
        dependency_graph: &mut JobDependencyGraph<S>,
        world_state: Arc<S>,
    ) -> Result<Self> {
        let state = Arc::new(JobExecutionState::new(
            job_pool,
            dependency_graph,
            world_state,
        )?);
        let thread_pool = ThreadPool::new(n_workers, &Self::execute_job);
        Ok(Self { state, thread_pool })
    }

    fn job_ordering(&self) -> &JobOrdering<S> {
        self.state.job_ordering()
    }

    fn execute_on_main_thread(&self) {
        for job_idx in 0..self.thread_pool.n_workers() {
            let job = self.job_ordering().job(job_idx);
            job.run(self.state.world_state()).expect("Job failed");
        }
    }

    fn execute(&self) {
        self.thread_pool.execute_with_workers(
            (0..self.job_ordering().n_dependencyless_jobs())
                .map(|job_idx| Self::create_message(&self.state, job_idx)),
        );
        self.thread_pool.wait_for_all_workers_idle();
        self.job_ordering().reset();
    }

    fn execute_job(
        communicator: &ThreadCommunicator<JobMessage<S>>,
        (state, job_idx): JobMessage<S>,
    ) {
        let job = state.job_ordering().job(job_idx);

        {
            cfg_if::cfg_if! {
                if #[cfg(test)] {
                    job.run_with_worker(communicator.worker_id(), state.world_state())
                } else {
                    job.run(state.world_state())
                }
            }
        }
        .expect("Job failed");

        let ready_dependent_job_indices: Vec<_> = job
            .indices_of_dependent_jobs()
            .iter()
            .cloned()
            .filter(|&dependent_job_idx| {
                let dependent_job = state.job_ordering().job(dependent_job_idx);
                let job_ready = dependent_job.complete_dependency();
                job_ready == JobReady::Yes
            })
            .collect();

        if ready_dependent_job_indices.len() > 1 {
            for &ready_dependent_job_idx in &ready_dependent_job_indices[1..] {
                communicator
                    .send_execute_message(Self::create_message(&state, ready_dependent_job_idx));
            }
        }
        if let Some(&ready_dependent_job_idx) = ready_dependent_job_indices.first() {
            Self::execute_job(communicator, (state, ready_dependent_job_idx))
        }
    }

    fn create_message(state: &Arc<JobExecutionState<S>>, job_idx: usize) -> JobMessage<S> {
        (Arc::clone(state), job_idx)
    }
}

impl<S> JobExecutionState<S> {
    fn new(
        job_pool: &JobPool<S>,
        dependency_graph: &mut JobDependencyGraph<S>,
        world_state: Arc<S>,
    ) -> Result<Self> {
        let job_ordering = JobOrdering::new(job_pool, dependency_graph)?;
        Ok(Self {
            job_ordering,
            world_state,
        })
    }

    fn job_ordering(&self) -> &JobOrdering<S> {
        &self.job_ordering
    }

    fn world_state(&self) -> &S {
        self.world_state.as_ref()
    }
}

impl<S> JobOrdering<S> {
    fn new(job_pool: &JobPool<S>, dependency_graph: &mut JobDependencyGraph<S>) -> Result<Self> {
        let jobs = Self::create_ordered_jobs(job_pool, dependency_graph)?;
        let n_dependencyless_jobs = Self::find_n_dependencyless_jobs(&jobs);
        Ok(Self {
            jobs,
            n_dependencyless_jobs,
        })
    }

    fn n_dependencyless_jobs(&self) -> usize {
        self.n_dependencyless_jobs
    }

    fn job(&self, idx: usize) -> &OrderedJob<S> {
        &self.jobs[idx]
    }

    fn reset(&self) {
        self.reset_completed_dependency_counts();
    }

    fn reset_completed_dependency_counts(&self) {
        for job in &self.jobs {
            job.reset_completed_dependency_count()
        }
    }

    fn create_ordered_jobs(
        job_pool: &JobPool<S>,
        dependency_graph: &mut JobDependencyGraph<S>,
    ) -> Result<Vec<OrderedJob<S>>> {
        let ordered_job_ids = dependency_graph.obtain_ordered_job_ids()?;

        // Create map from job ID to index in `ordered_job_ids`
        let indices_of_job_ids: HashMap<_, _> = ordered_job_ids
            .iter()
            .enumerate()
            .map(|(idx, &job_id)| (job_id, idx))
            .collect();

        ordered_job_ids
            .into_iter()
            .map(|job_id| {
                let job = job_pool
                    .get(&job_id)
                    .ok_or_else(|| anyhow!("Dependency job (ID {}) missing", job_id))?;

                // Find index into `ordered_job_ids` of each job
                // that depends on this job
                let indices_of_dependent_jobs = dependency_graph
                    .find_dependent_job_ids(job_id)
                    .map(|dependent_job_id| indices_of_job_ids[&dependent_job_id]);

                Ok(OrderedJob::new(Arc::clone(job), indices_of_dependent_jobs))
            })
            .collect()
    }

    fn find_n_dependencyless_jobs(jobs: &[OrderedJob<S>]) -> usize {
        for (idx, job) in jobs.iter().enumerate() {
            if job.n_dependencies() > 0 {
                return idx;
            }
        }
        0
    }
}

impl<S> OrderedJob<S> {
    fn new(job: Arc<dyn Job<S>>, indices_of_dependent_jobs: impl Iterator<Item = usize>) -> Self {
        let n_dependencies = job.depends_on().len();
        Self {
            job,
            n_dependencies,
            indices_of_dependent_jobs: indices_of_dependent_jobs.collect(),
            completed_dependency_count: AtomicUsize::new(0),
        }
    }

    fn n_dependencies(&self) -> usize {
        self.n_dependencies
    }

    fn indices_of_dependent_jobs(&self) -> &[usize] {
        &self.indices_of_dependent_jobs
    }

    fn reset_completed_dependency_count(&self) {
        self.completed_dependency_count.store(0, Ordering::Release);
    }

    fn complete_dependency(&self) -> JobReady {
        let previous_count = self
            .completed_dependency_count
            .fetch_add(1, Ordering::AcqRel);

        assert!(previous_count < self.n_dependencies());

        if previous_count + 1 == self.n_dependencies() {
            JobReady::Yes
        } else {
            JobReady::No
        }
    }

    fn run(&self, world_state: &S) -> Result<()> {
        self.job.run(world_state)
    }

    #[cfg(test)]
    fn run_with_worker(&self, _worker_id: WorkerID, world_state: &S) -> Result<()> {
        self.job.run_with_worker(_worker_id, world_state)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{iter, sync::Mutex, thread, time::Duration};

    struct JobRecorder {
        recorded_jobs: Mutex<Vec<(WorkerID, JobID)>>,
    }

    impl JobRecorder {
        fn new() -> Self {
            Self {
                recorded_jobs: Mutex::new(Vec::new()),
            }
        }

        fn get_recorded_worker_ids(&self) -> Vec<WorkerID> {
            self.recorded_jobs
                .lock()
                .unwrap()
                .iter()
                .map(|&(worker_id, _)| worker_id)
                .collect()
        }

        fn get_recorded_job_ids(&self) -> Vec<JobID> {
            self.recorded_jobs
                .lock()
                .unwrap()
                .iter()
                .map(|&(_, job_id)| job_id)
                .collect()
        }

        fn record_job(&self, worker_id: WorkerID, job_id: JobID) {
            self.recorded_jobs.lock().unwrap().push((worker_id, job_id));
            thread::sleep(Duration::from_millis(1));
        }
    }

    macro_rules! create_job_type {
        (name = $job:ident, deps = [$($deps:ty),*]) => {
            struct $job;

            impl $job {
                const ID: JobID = super::hash_job_name_to_id(stringify!($job));
            }

            impl Job<JobRecorder> for $job
            {
                fn id(&self) -> JobID {
                    Self::ID
                }

                fn depends_on(&self) -> &[JobID] {
                    &[$(<$deps>::ID),*]
                }

                fn run(&self, _job_recorder: &JobRecorder) -> Result<()> {
                    unreachable!()
                }

                fn run_with_worker(&self, worker_id: WorkerID, job_recorder: &JobRecorder) -> Result<()> {
                    Ok(job_recorder.record_job(worker_id, self.id()))
                }
            }
        };
    }

    create_job_type!(name = Job1, deps = []);
    create_job_type!(name = Job2, deps = []);
    create_job_type!(name = DepJob1, deps = [Job1]);
    create_job_type!(name = DepJob2, deps = [Job2]);
    create_job_type!(name = DepDepJob1, deps = [DepJob1]);
    create_job_type!(name = DepJob1Job2, deps = [Job1, Job2]);
    create_job_type!(name = DepDepJob1Job2, deps = [DepJob1, Job2]);
    create_job_type!(name = CircularJob1, deps = [CircularJob2]);
    create_job_type!(name = CircularJob2, deps = [CircularJob1]);

    type TestDispatcher = Dispatcher<JobRecorder>;
    type TestJobDependencyGraph = JobDependencyGraph<JobRecorder>;
    type TestOrderedJob = OrderedJob<JobRecorder>;

    fn create_dispatcher(n_workers: usize) -> TestDispatcher {
        Dispatcher::new(
            NonZeroUsize::new(n_workers).unwrap(),
            Arc::new(JobRecorder::new()),
        )
    }

    #[test]
    fn registering_jobs_in_dependency_order_works() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(Job1).unwrap();
        assert!(dispatcher.has_job(Job1::ID));

        dispatcher.register_job(Job2).unwrap();
        assert!(dispatcher.has_job(Job1::ID));
        assert!(dispatcher.has_job(Job2::ID));

        dispatcher.register_job(DepJob1).unwrap();
        assert!(dispatcher.has_job(Job1::ID));
        assert!(dispatcher.has_job(Job1::ID));
        assert!(dispatcher.has_job(DepJob1::ID));

        dispatcher.register_job(DepDepJob1Job2).unwrap();
        assert!(dispatcher.has_job(Job1::ID));
        assert!(dispatcher.has_job(Job1::ID));
        assert!(dispatcher.has_job(DepJob1::ID));
        assert!(dispatcher.has_job(DepDepJob1Job2::ID));

        dispatcher.complete_job_registration().unwrap();
    }

    #[test]
    fn registering_jobs_out_of_dependency_order_works() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(DepDepJob1Job2).unwrap();
        assert!(dispatcher.has_job(DepDepJob1Job2::ID));

        dispatcher.register_job(Job2).unwrap();
        assert!(dispatcher.has_job(DepDepJob1Job2::ID));
        assert!(dispatcher.has_job(Job2::ID));

        dispatcher.register_job(DepJob1).unwrap();
        assert!(dispatcher.has_job(DepDepJob1Job2::ID));
        assert!(dispatcher.has_job(Job2::ID));
        assert!(dispatcher.has_job(DepJob1::ID));

        dispatcher.register_job(Job1).unwrap();
        assert!(dispatcher.has_job(DepDepJob1Job2::ID));
        assert!(dispatcher.has_job(Job2::ID));
        assert!(dispatcher.has_job(DepJob1::ID));
        assert!(dispatcher.has_job(Job1::ID));

        dispatcher.complete_job_registration().unwrap();
    }

    #[test]
    fn registering_no_jobs_works() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.complete_job_registration().unwrap();
    }

    #[test]
    #[should_panic]
    fn registering_same_job_twice_fails() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(Job1).unwrap();
        dispatcher.register_job(Job2).unwrap();
        dispatcher.register_job(Job1).unwrap();
    }

    #[test]
    #[should_panic]
    fn completing_with_missing_dependency_fails() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(DepJob1).unwrap();
        dispatcher.complete_job_registration().unwrap();
    }

    #[test]
    #[should_panic]
    fn creating_circular_job_dependencies_fails() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(CircularJob1).unwrap();
        dispatcher.register_job(CircularJob2).unwrap();
        dispatcher.complete_job_registration().unwrap();
    }

    #[test]
    #[should_panic]
    fn executing_before_completing_job_reg_fails() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(Job1).unwrap();
        dispatcher.execute();
    }

    #[test]
    fn executing_jobs_works() {
        let mut dispatcher = create_dispatcher(2);
        dispatcher.register_job(DepDepJob1Job2).unwrap();
        dispatcher.register_job(Job2).unwrap();
        dispatcher.register_job(DepJob1).unwrap();
        dispatcher.register_job(Job1).unwrap();
        dispatcher.register_job(DepJob1Job2).unwrap();
        dispatcher.complete_job_registration().unwrap();

        dispatcher.execute();
        let recorded_worker_ids = dispatcher.world_state().get_recorded_worker_ids();
        let recorded_job_ids = dispatcher.world_state().get_recorded_job_ids();

        match recorded_job_ids[..] {
            [Job1::ID, Job2::ID, DepJob1::ID, DepJob1Job2::ID, DepDepJob1Job2::ID] => {}
            [Job2::ID, Job1::ID, DepJob1::ID, DepJob1Job2::ID, DepDepJob1Job2::ID] => {}
            [Job1::ID, Job2::ID, DepJob1Job2::ID, DepJob1::ID, DepDepJob1Job2::ID] => {}
            [Job2::ID, Job1::ID, DepJob1Job2::ID, DepJob1::ID, DepDepJob1Job2::ID] => {}
            [Job1::ID, Job2::ID, DepJob1::ID, DepDepJob1Job2::ID, DepJob1Job2::ID] => {}
            [Job2::ID, Job1::ID, DepJob1::ID, DepDepJob1Job2::ID, DepJob1Job2::ID] => {}
            _ => panic!("Incorrect job order"),
        }

        let sorted_worker_ids: Vec<_> = [
            Job1::ID,
            Job2::ID,
            DepJob1::ID,
            DepJob1Job2::ID,
            DepDepJob1Job2::ID,
        ]
        .iter()
        .map(|job_id| {
            recorded_worker_ids[recorded_job_ids.iter().position(|id| id == job_id).unwrap()]
        })
        .collect();

        // First, Job1 and Job2 should be executed independently.
        // Then DepJob1 and DepJob1Job2 should be executed
        // independently by the thread that executed Job1 and Job2,
        // respectively. DepDepJob1Job2 should execute last and
        // on the thread that executed DepJob1.
        match sorted_worker_ids[..] {
            [0, 1, 0, 1, 0] => {}
            [1, 0, 1, 0, 1] => {}
            _ => panic!("Incorrect worker contribution"),
        }
    }

    #[test]
    fn ordered_jobs_are_created_correctly() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(DepDepJob1).unwrap();
        dispatcher.register_job(Job1).unwrap();
        dispatcher.register_job(DepJob1).unwrap();
        dispatcher.complete_job_registration().unwrap();

        let job_ordering = dispatcher.executor().job_ordering();
        {
            let job_1 = job_ordering.job(0);
            assert_eq!(job_1.n_dependencies(), 0);
            assert_eq!(job_1.indices_of_dependent_jobs(), &[1]);
        }
        {
            let dep_job_1 = job_ordering.job(1);
            assert_eq!(dep_job_1.n_dependencies(), 1);
            assert_eq!(dep_job_1.indices_of_dependent_jobs(), &[2]);
        }
        {
            let dep_dep_job_1 = job_ordering.job(2);
            assert_eq!(dep_dep_job_1.n_dependencies(), 1);
            assert!(dep_dep_job_1.indices_of_dependent_jobs().is_empty());
        }
    }

    #[test]
    fn finding_n_dependencyless_jobs_works() {
        let mut dispatcher = create_dispatcher(1);
        dispatcher.register_job(DepDepJob1).unwrap();
        dispatcher.register_job(Job1).unwrap();
        dispatcher.register_job(DepJob1).unwrap();
        dispatcher.complete_job_registration().unwrap();
        assert_eq!(
            dispatcher.executor().job_ordering().n_dependencyless_jobs(),
            1
        );

        dispatcher.register_job(Job2).unwrap();
        dispatcher.register_job(DepDepJob1Job2).unwrap();
        dispatcher.complete_job_registration().unwrap();
        assert_eq!(
            dispatcher.executor().job_ordering().n_dependencyless_jobs(),
            2
        );
    }

    #[test]
    fn jobs_are_ordered_correctly() {
        let mut dependency_graph = TestJobDependencyGraph::new();
        dependency_graph.add_job(&DepDepJob1Job2).unwrap();
        dependency_graph.add_job(&Job1).unwrap();
        dependency_graph.add_job(&DepJob1).unwrap();
        dependency_graph.add_job(&DepJob1Job2).unwrap();
        dependency_graph.add_job(&Job2).unwrap();

        let ordered_job_ids = dependency_graph.obtain_ordered_job_ids().unwrap();

        match ordered_job_ids[..] {
            [Job1::ID, Job2::ID, DepJob1::ID, DepDepJob1Job2::ID, DepJob1Job2::ID] => {}
            [Job2::ID, Job1::ID, DepJob1::ID, DepDepJob1Job2::ID, DepJob1Job2::ID] => {}
            [Job1::ID, Job2::ID, DepJob1::ID, DepJob1Job2::ID, DepDepJob1Job2::ID] => {}
            [Job2::ID, Job1::ID, DepJob1::ID, DepJob1Job2::ID, DepDepJob1Job2::ID] => {}
            [Job1::ID, Job2::ID, DepJob1Job2::ID, DepJob1::ID, DepDepJob1Job2::ID] => {}
            [Job2::ID, Job1::ID, DepJob1Job2::ID, DepJob1::ID, DepDepJob1Job2::ID] => {}
            _ => panic!("Incorrect job order"),
        }
    }

    #[test]
    fn finding_dependent_job_ids_works() {
        let mut dependency_graph = TestJobDependencyGraph::new();
        dependency_graph.add_job(&DepJob1).unwrap();
        dependency_graph.add_job(&DepDepJob1Job2).unwrap();
        dependency_graph.add_job(&Job1).unwrap();
        dependency_graph.add_job(&DepJob1Job2).unwrap();
        dependency_graph.add_job(&Job2).unwrap();
        dependency_graph.add_job(&DepDepJob1).unwrap();

        let dependent_job_ids: Vec<_> = dependency_graph.find_dependent_job_ids(Job1::ID).collect();
        assert_eq!(dependent_job_ids.len(), 2);
        assert!(dependent_job_ids.contains(&DepJob1::ID));
        assert!(dependent_job_ids.contains(&DepJob1Job2::ID));

        let dependent_job_ids: Vec<_> = dependency_graph.find_dependent_job_ids(Job2::ID).collect();
        assert_eq!(dependent_job_ids.len(), 2);
        assert!(dependent_job_ids.contains(&DepJob1Job2::ID));
        assert!(dependent_job_ids.contains(&DepDepJob1Job2::ID));
    }

    #[test]
    fn completing_dependencies_of_ordered_job_works() {
        let ordered_job = TestOrderedJob::new(Arc::new(DepJob1Job2), iter::empty());
        assert_eq!(ordered_job.n_dependencies(), 2);
        assert_eq!(ordered_job.complete_dependency(), JobReady::No);
        assert_eq!(ordered_job.complete_dependency(), JobReady::Yes);
        ordered_job.reset_completed_dependency_count();
        assert_eq!(ordered_job.complete_dependency(), JobReady::No);
        assert_eq!(ordered_job.complete_dependency(), JobReady::Yes);
    }

    #[test]
    #[should_panic]
    fn completing_too_many_dependencies_of_ordered_job_fails() {
        let ordered_job = TestOrderedJob::new(Arc::new(Job1), iter::empty());
        ordered_job.complete_dependency();
    }
}
