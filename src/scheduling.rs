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

pub type TaskID = u64;

pub trait Task<S>: Sync + Send + std::fmt::Debug {
    fn id(&self) -> TaskID;

    fn depends_on(&self) -> &[TaskID];

    fn execute(&self, world_state: &S) -> Result<()>;

    #[cfg(test)]
    fn execute_with_worker(&self, _worker_id: WorkerID, world_state: &S) -> Result<()> {
        self.execute(world_state)
    }
}

type TaskSchedulerThreadPool<S> = ThreadPool<TaskMessage<S>>;
type TaskPool<S> = HashMap<TaskID, Arc<dyn Task<S>>>;
type TaskMessage<S> = (Arc<TaskExecutionState<S>>, usize);

#[derive(Debug)]
pub struct TaskScheduler<S> {
    n_workers: NonZeroUsize,
    tasks: TaskPool<S>,
    dependency_graph: TaskDependencyGraph<S>,
    executor: Option<TaskExecutor<S>>,
    world_state: Arc<S>,
}

#[derive(Debug)]
struct TaskDependencyGraph<S> {
    graph: DiGraphMap<TaskID, ()>,
    space: DfsSpace<TaskID, HashSet<TaskID>>,
    independent_tasks: HashSet<TaskID>,
    _phantom: PhantomData<S>,
}

#[derive(Debug)]
struct TaskExecutor<S> {
    state: Arc<TaskExecutionState<S>>,
    thread_pool: TaskSchedulerThreadPool<S>,
}

#[derive(Debug)]
struct TaskExecutionState<S> {
    task_ordering: TaskOrdering<S>,
    world_state: Arc<S>,
}

#[derive(Debug)]
struct TaskOrdering<S> {
    tasks: Vec<OrderedTask<S>>,
    n_dependencyless_tasks: usize,
}

#[derive(Debug)]
struct OrderedTask<S> {
    task: Arc<dyn Task<S>>,
    n_dependencies: usize,
    indices_of_dependent_tasks: Vec<usize>,
    completed_dependency_count: AtomicUsize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TaskReady {
    Yes,
    No,
}

pub const fn hash_task_name_to_id(name: &str) -> TaskID {
    const_fnv1a_hash::fnv1a_hash_str_64(name)
}

impl<S> TaskScheduler<S>
where
    S: Sync + Send + 'static,
{
    pub fn new(n_workers: NonZeroUsize, world_state: Arc<S>) -> Self {
        Self {
            n_workers,
            tasks: HashMap::new(),
            dependency_graph: TaskDependencyGraph::new(),
            executor: None,
            world_state,
        }
    }

    pub fn world_state(&self) -> &S {
        self.world_state.as_ref()
    }

    pub fn has_task(&self, task_id: TaskID) -> bool {
        self.tasks.contains_key(&task_id)
    }

    pub fn execute(&self) {
        self.executor
            .as_ref()
            .expect("Called `execute` before completing task registration")
            .execute();
    }

    pub fn execute_on_main_thread(&self) {
        self.executor
            .as_ref()
            .expect("Called `execute` before completing task registration")
            .execute_on_main_thread();
    }

    pub fn register_task(&mut self, task: impl Task<S> + 'static) -> Result<()> {
        let task_id = task.id();
        if self.tasks.contains_key(&task_id) {
            bail!("Task with ID {} already exists", task_id);
        }

        self.dependency_graph.add_task(&task)?;

        self.tasks.insert(task.id(), Arc::new(task));

        // Changing the tasks invalidates the executor
        self.executor = None;

        Ok(())
    }

    pub fn complete_task_registration(&mut self) -> Result<()> {
        self.executor = Some(TaskExecutor::new(
            self.n_workers,
            &self.tasks,
            &mut self.dependency_graph,
            Arc::clone(&self.world_state),
        )?);
        Ok(())
    }

    #[allow(dead_code)]
    fn executor(&self) -> &TaskExecutor<S> {
        self.executor.as_ref().unwrap()
    }
}

impl<S> TaskDependencyGraph<S> {
    fn new() -> Self {
        let graph = DiGraphMap::new();
        let space = DfsSpace::new(&graph);
        let independent_tasks = HashSet::new();
        Self {
            graph,
            space,
            independent_tasks,
            _phantom: PhantomData,
        }
    }

    fn add_task(&mut self, task: &impl Task<S>) -> Result<()> {
        let task_id = task.id();
        self.graph.add_node(task_id);

        let dependence_task_ids = task.depends_on();

        if dependence_task_ids.is_empty() {
            self.independent_tasks.insert(task_id);
        }

        for &dependence_task_id in dependence_task_ids {
            // Add edge directed from dependence to dependent.
            // A node for the dependence task is added if it
            // doesn't exist.
            let existing_edge = self.graph.add_edge(dependence_task_id, task_id, ());

            if existing_edge.is_some() {
                bail!(
                    "Task with ID {} depends on same task (ID {}) multiple times",
                    task_id,
                    dependence_task_id
                );
            }
        }
        Ok(())
    }

    fn obtain_ordered_task_ids(&mut self) -> Result<Vec<TaskID>> {
        let n_tasks = self.graph.node_count();
        let mut sorted_ids = Vec::with_capacity(n_tasks);

        // Make sure all tasks without dependencies come first
        sorted_ids.extend(self.independent_tasks.iter());

        if n_tasks > self.independent_tasks.len() {
            // Get task IDs sorted to topological order, meaning an order
            // where each task comes after all its dependencies
            let topologically_sorted_ids = algo::toposort(&self.graph, Some(&mut self.space))
                .map_err(|_| anyhow!("Found circular task dependencies"))?;

            // Add all tasks with dependencies in topological order
            sorted_ids.extend(
                topologically_sorted_ids
                    .into_iter()
                    .filter(|task_id| !self.independent_tasks.contains(task_id)),
            );
        }

        assert_eq!(sorted_ids.len(), n_tasks);

        Ok(sorted_ids)
    }

    fn find_dependent_task_ids(&self, task_id: TaskID) -> impl Iterator<Item = TaskID> + '_ {
        // Find outgoing edges, i.e. to tasks depending on this one
        self.graph
            .edges(task_id)
            .map(|(_task_id, dependent_task_id, _)| dependent_task_id)
    }
}

impl<S> TaskExecutor<S>
where
    S: Sync + Send + 'static,
{
    fn new(
        n_workers: NonZeroUsize,
        task_pool: &TaskPool<S>,
        dependency_graph: &mut TaskDependencyGraph<S>,
        world_state: Arc<S>,
    ) -> Result<Self> {
        let state = Arc::new(TaskExecutionState::new(
            task_pool,
            dependency_graph,
            world_state,
        )?);
        let thread_pool = ThreadPool::new(n_workers, &Self::execute_task);
        Ok(Self { state, thread_pool })
    }

    fn task_ordering(&self) -> &TaskOrdering<S> {
        self.state.task_ordering()
    }

    fn execute_on_main_thread(&self) {
        for task_idx in 0..self.thread_pool.n_workers() {
            let task = self.task_ordering().task(task_idx);
            task.execute(self.state.world_state()).expect("Task failed");
        }
    }

    fn execute(&self) {
        self.thread_pool.execute_with_workers(
            (0..self.task_ordering().n_dependencyless_tasks())
                .map(|task_idx| Self::create_message(&self.state, task_idx)),
        );
        self.thread_pool.wait_for_all_workers_idle();
        self.task_ordering().reset();
    }

    fn execute_task(
        communicator: &ThreadCommunicator<TaskMessage<S>>,
        (state, task_idx): TaskMessage<S>,
    ) {
        let task = state.task_ordering().task(task_idx);

        {
            cfg_if::cfg_if! {
                if #[cfg(test)] {
                    task.execute_with_worker(communicator.worker_id(), state.world_state())
                } else {
                    task.execute(state.world_state())
                }
            }
        }
        .expect("Task failed");

        let ready_dependent_task_indices: Vec<_> = task
            .indices_of_dependent_tasks()
            .iter()
            .cloned()
            .filter(|&dependent_task_idx| {
                let dependent_task = state.task_ordering().task(dependent_task_idx);
                let task_ready = dependent_task.complete_dependency();
                task_ready == TaskReady::Yes
            })
            .collect();

        if ready_dependent_task_indices.len() > 1 {
            for &ready_dependent_task_idx in &ready_dependent_task_indices[1..] {
                communicator
                    .send_execute_message(Self::create_message(&state, ready_dependent_task_idx));
            }
        }
        if let Some(&ready_dependent_task_idx) = ready_dependent_task_indices.first() {
            Self::execute_task(communicator, (state, ready_dependent_task_idx))
        }
    }

    fn create_message(state: &Arc<TaskExecutionState<S>>, task_idx: usize) -> TaskMessage<S> {
        (Arc::clone(state), task_idx)
    }
}

impl<S> TaskExecutionState<S> {
    fn new(
        task_pool: &TaskPool<S>,
        dependency_graph: &mut TaskDependencyGraph<S>,
        world_state: Arc<S>,
    ) -> Result<Self> {
        let task_ordering = TaskOrdering::new(task_pool, dependency_graph)?;
        Ok(Self {
            task_ordering,
            world_state,
        })
    }

    fn task_ordering(&self) -> &TaskOrdering<S> {
        &self.task_ordering
    }

    fn world_state(&self) -> &S {
        self.world_state.as_ref()
    }
}

impl<S> TaskOrdering<S> {
    fn new(task_pool: &TaskPool<S>, dependency_graph: &mut TaskDependencyGraph<S>) -> Result<Self> {
        let tasks = Self::create_ordered_tasks(task_pool, dependency_graph)?;
        let n_dependencyless_tasks = Self::find_n_dependencyless_tasks(&tasks);
        Ok(Self {
            tasks,
            n_dependencyless_tasks,
        })
    }

    fn n_dependencyless_tasks(&self) -> usize {
        self.n_dependencyless_tasks
    }

    fn task(&self, idx: usize) -> &OrderedTask<S> {
        &self.tasks[idx]
    }

    fn reset(&self) {
        self.reset_completed_dependency_counts();
    }

    fn reset_completed_dependency_counts(&self) {
        for task in &self.tasks {
            task.reset_completed_dependency_count()
        }
    }

    fn create_ordered_tasks(
        task_pool: &TaskPool<S>,
        dependency_graph: &mut TaskDependencyGraph<S>,
    ) -> Result<Vec<OrderedTask<S>>> {
        let ordered_task_ids = dependency_graph.obtain_ordered_task_ids()?;

        // Create map from task ID to index in `ordered_task_ids`
        let indices_of_task_ids: HashMap<_, _> = ordered_task_ids
            .iter()
            .enumerate()
            .map(|(idx, &task_id)| (task_id, idx))
            .collect();

        ordered_task_ids
            .into_iter()
            .map(|task_id| {
                let task = task_pool
                    .get(&task_id)
                    .ok_or_else(|| anyhow!("Dependency task (ID {}) missing", task_id))?;

                // Find index into `ordered_task_ids` of each task
                // that depends on this task
                let indices_of_dependent_tasks = dependency_graph
                    .find_dependent_task_ids(task_id)
                    .map(|dependent_task_id| indices_of_task_ids[&dependent_task_id]);

                Ok(OrderedTask::new(
                    Arc::clone(task),
                    indices_of_dependent_tasks,
                ))
            })
            .collect()
    }

    fn find_n_dependencyless_tasks(tasks: &[OrderedTask<S>]) -> usize {
        for (idx, task) in tasks.iter().enumerate() {
            if task.n_dependencies() > 0 {
                return idx;
            }
        }
        0
    }
}

impl<S> OrderedTask<S> {
    fn new(
        task: Arc<dyn Task<S>>,
        indices_of_dependent_tasks: impl Iterator<Item = usize>,
    ) -> Self {
        let n_dependencies = task.depends_on().len();
        Self {
            task,
            n_dependencies,
            indices_of_dependent_tasks: indices_of_dependent_tasks.collect(),
            completed_dependency_count: AtomicUsize::new(0),
        }
    }

    fn n_dependencies(&self) -> usize {
        self.n_dependencies
    }

    fn indices_of_dependent_tasks(&self) -> &[usize] {
        &self.indices_of_dependent_tasks
    }

    fn reset_completed_dependency_count(&self) {
        self.completed_dependency_count.store(0, Ordering::Release);
    }

    fn complete_dependency(&self) -> TaskReady {
        let previous_count = self
            .completed_dependency_count
            .fetch_add(1, Ordering::AcqRel);

        assert!(previous_count < self.n_dependencies());

        if previous_count + 1 == self.n_dependencies() {
            TaskReady::Yes
        } else {
            TaskReady::No
        }
    }

    fn execute(&self, world_state: &S) -> Result<()> {
        self.task.execute(world_state)
    }

    #[cfg(test)]
    fn execute_with_worker(&self, _worker_id: WorkerID, world_state: &S) -> Result<()> {
        self.task.execute_with_worker(_worker_id, world_state)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{iter, sync::Mutex, thread, time::Duration};

    struct TaskRecorder {
        recorded_tasks: Mutex<Vec<(WorkerID, TaskID)>>,
    }

    impl TaskRecorder {
        fn new() -> Self {
            Self {
                recorded_tasks: Mutex::new(Vec::new()),
            }
        }

        fn get_recorded_worker_ids(&self) -> Vec<WorkerID> {
            self.recorded_tasks
                .lock()
                .unwrap()
                .iter()
                .map(|&(worker_id, _)| worker_id)
                .collect()
        }

        fn get_recorded_task_ids(&self) -> Vec<TaskID> {
            self.recorded_tasks
                .lock()
                .unwrap()
                .iter()
                .map(|&(_, task_id)| task_id)
                .collect()
        }

        fn record_task(&self, worker_id: WorkerID, task_id: TaskID) {
            self.recorded_tasks
                .lock()
                .unwrap()
                .push((worker_id, task_id));
            thread::sleep(Duration::from_millis(1));
        }
    }

    macro_rules! create_task_type {
        (name = $task:ident, deps = [$($deps:ty),*]) => {
            #[derive(Debug)]
            struct $task;

            impl $task {
                const ID: TaskID = super::hash_task_name_to_id(stringify!($task));
            }

            impl Task<TaskRecorder> for $task
            {
                fn id(&self) -> TaskID {
                    Self::ID
                }

                fn depends_on(&self) -> &[TaskID] {
                    &[$(<$deps>::ID),*]
                }

                fn execute(&self, _task_recorder: &TaskRecorder) -> Result<()> {
                    unreachable!()
                }

                fn execute_with_worker(&self, worker_id: WorkerID, task_recorder: &TaskRecorder) -> Result<()> {
                    Ok(task_recorder.record_task(worker_id, self.id()))
                }
            }
        };
    }

    create_task_type!(name = Task1, deps = []);
    create_task_type!(name = Task2, deps = []);
    create_task_type!(name = DepTask1, deps = [Task1]);
    create_task_type!(name = DepTask2, deps = [Task2]);
    create_task_type!(name = DepDepTask1, deps = [DepTask1]);
    create_task_type!(name = DepTask1Task2, deps = [Task1, Task2]);
    create_task_type!(name = DepDepTask1Task2, deps = [DepTask1, Task2]);
    create_task_type!(name = CircularTask1, deps = [CircularTask2]);
    create_task_type!(name = CircularTask2, deps = [CircularTask1]);

    type TestTaskScheduler = TaskScheduler<TaskRecorder>;
    type TestTaskDependencyGraph = TaskDependencyGraph<TaskRecorder>;
    type TestOrderedTask = OrderedTask<TaskRecorder>;

    fn create_scheduler(n_workers: usize) -> TestTaskScheduler {
        TaskScheduler::new(
            NonZeroUsize::new(n_workers).unwrap(),
            Arc::new(TaskRecorder::new()),
        )
    }

    #[test]
    fn registering_tasks_in_dependency_order_works() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(Task1).unwrap();
        assert!(scheduler.has_task(Task1::ID));

        scheduler.register_task(Task2).unwrap();
        assert!(scheduler.has_task(Task1::ID));
        assert!(scheduler.has_task(Task2::ID));

        scheduler.register_task(DepTask1).unwrap();
        assert!(scheduler.has_task(Task1::ID));
        assert!(scheduler.has_task(Task1::ID));
        assert!(scheduler.has_task(DepTask1::ID));

        scheduler.register_task(DepDepTask1Task2).unwrap();
        assert!(scheduler.has_task(Task1::ID));
        assert!(scheduler.has_task(Task1::ID));
        assert!(scheduler.has_task(DepTask1::ID));
        assert!(scheduler.has_task(DepDepTask1Task2::ID));

        scheduler.complete_task_registration().unwrap();
    }

    #[test]
    fn registering_tasks_out_of_dependency_order_works() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(DepDepTask1Task2).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2::ID));

        scheduler.register_task(Task2).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2::ID));
        assert!(scheduler.has_task(Task2::ID));

        scheduler.register_task(DepTask1).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2::ID));
        assert!(scheduler.has_task(Task2::ID));
        assert!(scheduler.has_task(DepTask1::ID));

        scheduler.register_task(Task1).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2::ID));
        assert!(scheduler.has_task(Task2::ID));
        assert!(scheduler.has_task(DepTask1::ID));
        assert!(scheduler.has_task(Task1::ID));

        scheduler.complete_task_registration().unwrap();
    }

    #[test]
    fn registering_no_tasks_works() {
        let mut scheduler = create_scheduler(1);
        scheduler.complete_task_registration().unwrap();
    }

    #[test]
    #[should_panic]
    fn registering_same_task_twice_fails() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(Task1).unwrap();
        scheduler.register_task(Task2).unwrap();
        scheduler.register_task(Task1).unwrap();
    }

    #[test]
    #[should_panic]
    fn completing_with_missing_dependency_fails() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(DepTask1).unwrap();
        scheduler.complete_task_registration().unwrap();
    }

    #[test]
    #[should_panic]
    fn creating_circular_task_dependencies_fails() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(CircularTask1).unwrap();
        scheduler.register_task(CircularTask2).unwrap();
        scheduler.complete_task_registration().unwrap();
    }

    #[test]
    #[should_panic]
    fn executing_before_completing_task_reg_fails() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(Task1).unwrap();
        scheduler.execute();
    }

    #[test]
    fn executing_tasks_works() {
        let mut scheduler = create_scheduler(2);
        scheduler.register_task(DepDepTask1Task2).unwrap();
        scheduler.register_task(Task2).unwrap();
        scheduler.register_task(DepTask1).unwrap();
        scheduler.register_task(Task1).unwrap();
        scheduler.register_task(DepTask1Task2).unwrap();
        scheduler.complete_task_registration().unwrap();

        scheduler.execute();
        let recorded_worker_ids = scheduler.world_state().get_recorded_worker_ids();
        let recorded_task_ids = scheduler.world_state().get_recorded_task_ids();

        match recorded_task_ids[..] {
            [Task1::ID, Task2::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID] => {}
            [Task2::ID, Task1::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID] => {}
            [Task1::ID, Task2::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID] => {}
            [Task2::ID, Task1::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID] => {}
            [Task1::ID, Task2::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID] => {}
            [Task2::ID, Task1::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID] => {}
            _ => panic!("Incorrect task order"),
        }

        let sorted_worker_ids: Vec<_> = [
            Task1::ID,
            Task2::ID,
            DepTask1::ID,
            DepTask1Task2::ID,
            DepDepTask1Task2::ID,
        ]
        .iter()
        .map(|task_id| {
            recorded_worker_ids[recorded_task_ids
                .iter()
                .position(|id| id == task_id)
                .unwrap()]
        })
        .collect();

        // First, Task1 and Task2 should be executed independently.
        // Then DepTask1 and DepTask1Task2 should be executed
        // independently by the thread that executed Task1 and Task2,
        // respectively. DepDepTask1Task2 should execute last and
        // on the thread that executed DepTask1.
        match sorted_worker_ids[..] {
            [0, 1, 0, 1, 0] => {}
            [1, 0, 1, 0, 1] => {}
            _ => panic!("Incorrect worker contribution"),
        }
    }

    #[test]
    fn ordered_tasks_are_created_correctly() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(DepDepTask1).unwrap();
        scheduler.register_task(Task1).unwrap();
        scheduler.register_task(DepTask1).unwrap();
        scheduler.complete_task_registration().unwrap();

        let task_ordering = scheduler.executor().task_ordering();
        {
            let task_1 = task_ordering.task(0);
            assert_eq!(task_1.n_dependencies(), 0);
            assert_eq!(task_1.indices_of_dependent_tasks(), &[1]);
        }
        {
            let dep_task_1 = task_ordering.task(1);
            assert_eq!(dep_task_1.n_dependencies(), 1);
            assert_eq!(dep_task_1.indices_of_dependent_tasks(), &[2]);
        }
        {
            let dep_dep_task_1 = task_ordering.task(2);
            assert_eq!(dep_dep_task_1.n_dependencies(), 1);
            assert!(dep_dep_task_1.indices_of_dependent_tasks().is_empty());
        }
    }

    #[test]
    fn finding_n_dependencyless_tasks_works() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(DepDepTask1).unwrap();
        scheduler.register_task(Task1).unwrap();
        scheduler.register_task(DepTask1).unwrap();
        scheduler.complete_task_registration().unwrap();
        assert_eq!(
            scheduler
                .executor()
                .task_ordering()
                .n_dependencyless_tasks(),
            1
        );

        scheduler.register_task(Task2).unwrap();
        scheduler.register_task(DepDepTask1Task2).unwrap();
        scheduler.complete_task_registration().unwrap();
        assert_eq!(
            scheduler
                .executor()
                .task_ordering()
                .n_dependencyless_tasks(),
            2
        );
    }

    #[test]
    fn tasks_are_ordered_correctly() {
        let mut dependency_graph = TestTaskDependencyGraph::new();
        dependency_graph.add_task(&DepDepTask1Task2).unwrap();
        dependency_graph.add_task(&Task1).unwrap();
        dependency_graph.add_task(&DepTask1).unwrap();
        dependency_graph.add_task(&DepTask1Task2).unwrap();
        dependency_graph.add_task(&Task2).unwrap();

        let ordered_task_ids = dependency_graph.obtain_ordered_task_ids().unwrap();

        match ordered_task_ids[..] {
            [Task1::ID, Task2::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID] => {}
            [Task2::ID, Task1::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID] => {}
            [Task1::ID, Task2::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID] => {}
            [Task2::ID, Task1::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID] => {}
            [Task1::ID, Task2::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID] => {}
            [Task2::ID, Task1::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID] => {}
            _ => panic!("Incorrect task order"),
        }
    }

    #[test]
    fn finding_dependent_task_ids_works() {
        let mut dependency_graph = TestTaskDependencyGraph::new();
        dependency_graph.add_task(&DepTask1).unwrap();
        dependency_graph.add_task(&DepDepTask1Task2).unwrap();
        dependency_graph.add_task(&Task1).unwrap();
        dependency_graph.add_task(&DepTask1Task2).unwrap();
        dependency_graph.add_task(&Task2).unwrap();
        dependency_graph.add_task(&DepDepTask1).unwrap();

        let dependent_task_ids: Vec<_> = dependency_graph
            .find_dependent_task_ids(Task1::ID)
            .collect();
        assert_eq!(dependent_task_ids.len(), 2);
        assert!(dependent_task_ids.contains(&DepTask1::ID));
        assert!(dependent_task_ids.contains(&DepTask1Task2::ID));

        let dependent_task_ids: Vec<_> = dependency_graph
            .find_dependent_task_ids(Task2::ID)
            .collect();
        assert_eq!(dependent_task_ids.len(), 2);
        assert!(dependent_task_ids.contains(&DepTask1Task2::ID));
        assert!(dependent_task_ids.contains(&DepDepTask1Task2::ID));
    }

    #[test]
    fn completing_dependencies_of_ordered_task_works() {
        let ordered_task = TestOrderedTask::new(Arc::new(DepTask1Task2), iter::empty());
        assert_eq!(ordered_task.n_dependencies(), 2);
        assert_eq!(ordered_task.complete_dependency(), TaskReady::No);
        assert_eq!(ordered_task.complete_dependency(), TaskReady::Yes);
        ordered_task.reset_completed_dependency_count();
        assert_eq!(ordered_task.complete_dependency(), TaskReady::No);
        assert_eq!(ordered_task.complete_dependency(), TaskReady::Yes);
    }

    #[test]
    #[should_panic]
    fn completing_too_many_dependencies_of_ordered_task_fails() {
        let ordered_task = TestOrderedTask::new(Arc::new(Task1), iter::empty());
        ordered_task.complete_dependency();
    }
}
