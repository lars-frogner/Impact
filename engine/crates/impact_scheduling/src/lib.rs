//! Task scheduling.

#[macro_use]
pub mod macros;

use anyhow::{Result, anyhow, bail};
use impact_containers::{HashMap, HashSet, RandomState};
use impact_math::Hash64;
use impact_thread::{
    TaskClosureReturnValue, TaskError, TaskID, ThreadPool, ThreadPoolChannel, ThreadPoolResult,
};
use petgraph::{
    Directed,
    algo::{self, DfsSpace},
    graphmap::GraphMap,
};
use std::{
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Represents a piece of work to be performed by a worker thread in a
/// [`TaskScheduler`].
///
/// # Type parameters `S` is the type of an object representing the external
/// state that the task can modify.
pub trait Task<S>: Sync + Send + Debug {
    /// Returns a unique ID identifying this task.
    fn id(&self) -> TaskID;

    /// Returns the ID of every other task that must have been completed before
    /// this task can be executed.
    fn depends_on(&self) -> &[TaskID];

    /// Executes the task and modifies the given state accordingly. This method
    /// may fail and return an error.
    fn execute(&self, external_state: &S) -> Result<(), TaskError>;

    /// Whether this task should be included in a [`TaskScheduler`] execution
    /// tagged with the given tags.
    fn should_execute(&self, execution_tags: &ExecutionTags) -> bool;

    /// Like [`execute`](Self::execute), but the ID of the worker executing the
    /// task is included as an argument. Useful for testing.
    #[cfg(test)]
    fn execute_with_worker(
        &self,
        _worker_id: impact_thread::WorkerID,
        external_state: &S,
    ) -> Result<()> {
        self.execute(external_state)
    }
}

/// A task manager that can schedule execution of multiple
/// interdependent tasks.
#[derive(Debug)]
pub struct TaskScheduler<S> {
    n_workers: NonZeroUsize,
    queue_capacity: NonZeroUsize,
    tasks: TaskPool<S>,
    dependency_graph: TaskDependencyGraph<S>,
    executor: Option<TaskExecutor<S>>,
    external_state: S,
}

/// A tag associated with an execution of a [`TaskScheduler`].
pub type ExecutionTag = Hash64;

/// A set of unique [`ExecutionTag`]s.
pub type ExecutionTags = HashSet<ExecutionTag>;

type TaskPool<S> = HashMap<TaskID, Arc<dyn Task<S>>>;

/// Type of message sent to worker threads in a [`TaskScheduler`].
type TaskMessage<S> = (
    Arc<TaskExecutionState<S>>,
    Arc<HashSet<ExecutionTag>>,
    usize,
);

type TaskSchedulerThreadPool<S> = ThreadPool<TaskMessage<S>>;

/// A graph describing the dependencies between separate tasks.
#[derive(Debug)]
struct TaskDependencyGraph<S> {
    graph: GraphMap<TaskID, (), Directed, RandomState>,
    space: DfsSpace<TaskID, hashbrown::HashSet<TaskID>>,
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
    external_state: S,
}

/// A list of tasks ordered according to the following criteria:
/// - All tasks without dependencies come first in the list.
/// - Every task comes after all of the tasks it depends on.
#[derive(Debug)]
struct TaskOrdering<S> {
    tasks: Vec<OrderedTask<S>>,
    n_dependencyless_tasks: usize,
}

/// A wrapper for a [`Task`] inside a [`TaskOrdering`] that
/// includes some dependency information and state.
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

impl<S> TaskScheduler<S>
where
    S: Sync + Send + Clone + 'static,
{
    /// Creates a new task scheduler that will operate with the given number of
    /// worker threads on the given external state.
    pub fn new(n_workers: NonZeroUsize, queue_capacity: NonZeroUsize, external_state: S) -> Self {
        Self {
            n_workers,
            queue_capacity,
            tasks: HashMap::default(),
            dependency_graph: TaskDependencyGraph::new(),
            executor: None,
            external_state,
        }
    }

    /// Returns the number of worker threads that will be used to
    /// execute tasks.
    pub fn n_workers(&self) -> usize {
        self.n_workers.get()
    }

    /// Returns the external state that the tasks can modify.
    pub fn external_state(&self) -> &S {
        &self.external_state
    }

    /// Whether the given task is registered in the scheduler.
    pub fn has_task(&self, task: impl Task<S>) -> bool {
        self.has_task_with_id(task.id())
    }

    /// Whether a task with the given ID is registered in the
    /// scheduler.
    pub fn has_task_with_id(&self, task_id: TaskID) -> bool {
        self.tasks.contains_key(&task_id)
    }

    /// Includes the given task in the pool of tasks that can be
    /// scheduled for execution. The tasks that the given task
    /// depends on do not have to be registered yet, but they
    /// must have been registered prior to calling
    /// [`complete_task_registration`](Self::complete_task_registration).
    ///
    /// # Errors
    /// Returns an error if the given task has already been
    /// registered.
    pub fn register_task(&mut self, task: impl Task<S> + 'static) -> Result<()> {
        let task_id = task.id();
        if self.tasks.contains_key(&task_id) {
            bail!("Task {} already exists", task_id);
        }

        self.dependency_graph.add_task(&task);

        self.tasks.insert(task.id(), Arc::new(task));

        // Changing the tasks invalidates the executor
        self.executor = None;

        Ok(())
    }

    /// Processes all registered tasks in preparation for execution.
    /// Must be called between [`register_task`](Self::register_task)
    /// and [`execute`](Self::execute) (or one of its variants),
    /// otherwise the execution call will panic.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Any of a task's dependencies have not been registered.
    /// - The tasks have circular dependencies.
    pub fn complete_task_registration(&mut self) -> Result<()> {
        self.executor = Some(TaskExecutor::new(
            self.n_workers,
            self.queue_capacity,
            &self.tasks,
            &mut self.dependency_graph,
            self.external_state.clone(),
        )?);
        Ok(())
    }

    /// Executes all tasks that [`should_execute`](Task::should_execute)
    /// for to the given execution tags on the main (calling) thread.
    ///
    /// Each task is executed after all its dependencies, but will not
    /// refrain from executing if a dependency does not execute due to
    /// the execution tags.
    ///
    /// # Panics
    /// If [`complete_task_registration`](Self::complete_task_registration)
    /// has not been called after the last task was registered.
    pub fn execute_on_main_thread(&self, execution_tags: &ExecutionTags) {
        self.executor
            .as_ref()
            .expect("Called `execute_on_main_thread` before completing task registration")
            .execute_on_main_thread(execution_tags);
    }

    /// Executes all tasks that [`should_execute`](Task::should_execute)
    /// for to the given execution tags, using [`n_workers`](Self::n_workers)
    /// worker threads.
    ///
    /// Each task is executed after all its dependencies, but will not
    /// refrain from executing if a dependency does not execute due to
    /// the execution tags.
    ///
    /// This function does not return until all tasks have been completed
    /// or have failed with an error. To avoid blocking the calling thread,
    /// use [`execute`](Self::execute) instead.
    ///
    /// # Errors
    /// A [`ThreadPoolTaskErrors`](impact_thread::ThreadPoolTaskErrors)
    /// containing the [`TaskError`] of each failed task is returned if any of
    /// the executed tasks failed.
    ///
    /// # Panics
    /// If [`complete_task_registration`](Self::complete_task_registration)
    /// has not been called after the last task was registered.
    pub fn execute_and_wait(&self, execution_tags: &Arc<ExecutionTags>) -> ThreadPoolResult {
        self.executor
            .as_ref()
            .expect("Called `execute_and_wait` before completing task registration")
            .execute_and_wait(execution_tags)
    }

    /// Executes all tasks that [`should_execute`](Task::should_execute)
    /// for to the given execution tags, using [`n_workers`](Self::n_workers)
    /// worker threads.
    ///
    /// Each task is executed after all its dependencies, but will not
    /// refrain from executing if a dependency does not execute due to
    /// the execution tags.
    ///
    /// This function returns as soon as the execution has been initiated.
    /// To block the calling thread until all tasks have been completed,
    /// call [`wait_until_done`](Self::wait_until_done).
    ///
    /// # Panics
    /// If [`complete_task_registration`](Self::complete_task_registration)
    /// has not been called after the last task was registered.
    pub fn execute(&self, execution_tags: &Arc<ExecutionTags>) {
        self.executor
            .as_ref()
            .expect("Called `execute` before completing task registration")
            .execute(execution_tags);
    }

    /// Blocks the calling thread and returns as soon as all tasks
    /// to be performed by the previous [`execute`](Self::execute)
    /// call have been completed or have failed with an error.
    ///
    /// # Errors
    /// A [`ThreadPoolTaskErrors`](impact_thread::ThreadPoolTaskErrors)
    /// containing the [`TaskError`] of each failed task is returned if any of
    /// the executed tasks failed.
    ///
    /// # Panics
    /// If [`complete_task_registration`](Self::complete_task_registration)
    /// has not been called after the last task was registered.
    pub fn wait_until_done(&self) -> ThreadPoolResult {
        self.executor
            .as_ref()
            .expect("Called `wait_until_done` before completing task registration")
            .wait_until_done()
    }

    #[cfg(test)]
    fn get_executor(&self) -> Option<&TaskExecutor<S>> {
        self.executor.as_ref()
    }
}

impl<S> TaskDependencyGraph<S> {
    fn new() -> Self {
        let graph = GraphMap::new();
        let space = DfsSpace::new(&graph);
        let independent_tasks = HashSet::default();
        Self {
            graph,
            space,
            independent_tasks,
            _phantom: PhantomData,
        }
    }

    fn add_task(&mut self, task: &impl Task<S>) {
        let task_id = task.id();
        self.graph.add_node(task_id);

        let dependence_task_ids = task.depends_on();

        for &dependence_task_id in dependence_task_ids {
            // Add edge directed from dependence to dependent.
            // A node for the dependence task is added if it
            // doesn't exist.
            let existing_edge = self.graph.add_edge(dependence_task_id, task_id, ());

            if existing_edge.is_some() {
                panic!("Task {task_id} depends on same task ({dependence_task_id}) multiple times");
            }
        }

        // Keep track of independent tasks separately as well
        if dependence_task_ids.is_empty() {
            self.independent_tasks.insert(task_id);
        }
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
                .map_err(|_cycle| anyhow!("Found circular task dependencies"))?;

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

    fn find_dependent_task_ids(&self, task_id: TaskID) -> impl Iterator<Item = TaskID> {
        // Find outgoing edges, i.e. to tasks depending on this one
        self.graph
            .edges(task_id)
            .map(|(_task_id, dependent_task_id, _)| dependent_task_id)
    }
}

impl<S> TaskExecutor<S>
where
    S: Sync + Send + Clone + 'static,
{
    fn new(
        n_workers: NonZeroUsize,
        queue_capacity: NonZeroUsize,
        task_pool: &TaskPool<S>,
        dependency_graph: &mut TaskDependencyGraph<S>,
        external_state: S,
    ) -> Result<Self> {
        let state = Arc::new(TaskExecutionState::new(
            task_pool,
            dependency_graph,
            external_state,
        )?);
        let thread_pool = ThreadPool::new(
            n_workers,
            queue_capacity,
            &Self::execute_task_and_schedule_dependencies,
        );
        Ok(Self { state, thread_pool })
    }

    fn task_ordering(&self) -> &TaskOrdering<S> {
        self.state.task_ordering()
    }

    fn execute_on_main_thread(&self, execution_tags: &ExecutionTags) {
        // Iterate through all tasks in order and execute each task
        // that should be
        for ordered_task in self.task_ordering().tasks() {
            let task = ordered_task.task();
            if task.should_execute(execution_tags) {
                task.execute(self.state.external_state())
                    .expect("Task failed");
            }
        }
    }

    fn execute_and_wait(&self, execution_tags: &Arc<ExecutionTags>) -> ThreadPoolResult {
        self.execute(execution_tags);
        self.wait_until_done()
    }

    fn execute(&self, execution_tags: &Arc<ExecutionTags>) {
        // Make sure that the count of completed dependencies
        // for each task is zeroed
        self.task_ordering().reset();

        // Start by scheduling all independent tasks (the ones at
        // the beginning of the ordered list of tasks) for immediate
        // execution. The execution of their dependencies will be
        // scheduled by the worker threads.
        self.thread_pool.execute(
            (0..self.task_ordering().n_dependencyless_tasks())
                .map(|task_idx| Self::create_message(&self.state, execution_tags, task_idx)),
        );
    }

    fn wait_until_done(&self) -> ThreadPoolResult {
        self.thread_pool.wait_until_done()
    }

    /// This is the function called by worker threads in the
    /// [`ThreadPool`] when they recieve an execution instruction.
    fn execute_task_and_schedule_dependencies(
        channel: &ThreadPoolChannel<TaskMessage<S>>,
        (state, execution_tags, task_idx): TaskMessage<S>,
    ) -> TaskClosureReturnValue {
        let ordered_task = state.task_ordering().task(task_idx);
        let task = ordered_task.task();

        impact_log::trace!(
            "Worker {} obtained task {}",
            channel.owning_worker_id(),
            task.id()
        );

        // Execute the task only if it thinks it should be based on
        // the current execution tags
        if task.should_execute(execution_tags.as_ref()) {
            impact_log::with_trace_logging!("Worker {} executing task {}",
                channel.owning_worker_id(),
                task.id();
                {
                    let result = {
                        cfg_if::cfg_if! {
                            if #[cfg(test)] {
                                task.execute_with_worker(channel.owning_worker_id(), state.external_state())
                            } else {
                                task.execute(state.external_state())
                            }
                        }
                    };

                    if let Err(error) = result {
                        // Return immediately with the task ID and an error
                        // if the task execution failed
                        return TaskClosureReturnValue::failure(task.id(), error);
                    }
                }
            );
        } else {
            impact_log::trace!(
                "Worker {} skipped execution of task {}",
                channel.owning_worker_id(),
                task.id()
            );
        }

        // Find each of the tasks that depend on this one, and
        // increment its count of completed dependencies. We keep
        // track of any dependent tasks that have no uncompleted
        // dependencies left as a result of completing this task.
        let mut ready_dependent_task_indices = ordered_task
            .indices_of_dependent_tasks()
            .iter()
            .filter_map(|&dependent_task_idx| {
                let dependent_task = state.task_ordering().task(dependent_task_idx);
                let task_ready = dependent_task.complete_dependency();
                if task_ready == TaskReady::Yes {
                    Some(dependent_task_idx)
                } else {
                    None
                }
            });

        // Take the first ready task for this thread to start executing
        // immediately
        let first_ready_dependent_task_idx = ready_dependent_task_indices.next();

        let mut n_additional_tasks = 0;

        // Schedule each remaining ready task
        for ready_dependent_task_idx in ready_dependent_task_indices {
            impact_log::with_trace_logging!(
                "Worker {} scheduling execution of task {}",
                channel.owning_worker_id(),
                state
                    .task_ordering()
                    .task(ready_dependent_task_idx)
                    .task()
                    .id();
                {
                    n_additional_tasks += 1;
                    channel.send_execute_instruction(Self::create_message(
                        &state,
                        &execution_tags,
                        ready_dependent_task_idx,
                    ));
                }
            );
        }

        if let Some(ready_dependent_task_idx) = first_ready_dependent_task_idx {
            n_additional_tasks += 1;
            Self::execute_task_and_schedule_dependencies(
                channel,
                (state, execution_tags, ready_dependent_task_idx),
            )
            // Add the counts from this task execution to those returned from
            // the dependent task
            .add(1, n_additional_tasks)
        } else {
            TaskClosureReturnValue::success(n_additional_tasks)
        }
    }

    fn create_message(
        state: &Arc<TaskExecutionState<S>>,
        execution_tags: &Arc<ExecutionTags>,
        task_idx: usize,
    ) -> TaskMessage<S> {
        (Arc::clone(state), Arc::clone(execution_tags), task_idx)
    }
}

impl<S> TaskExecutionState<S> {
    fn new(
        task_pool: &TaskPool<S>,
        dependency_graph: &mut TaskDependencyGraph<S>,
        external_state: S,
    ) -> Result<Self> {
        let task_ordering = TaskOrdering::new(task_pool, dependency_graph)?;
        Ok(Self {
            task_ordering,
            external_state,
        })
    }

    fn task_ordering(&self) -> &TaskOrdering<S> {
        &self.task_ordering
    }

    fn external_state(&self) -> &S {
        &self.external_state
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

    fn tasks(&self) -> &[OrderedTask<S>] {
        &self.tasks
    }

    fn reset(&self) {
        self.reset_completed_dependency_counts();
    }

    fn reset_completed_dependency_counts(&self) {
        for task in &self.tasks {
            task.reset_completed_dependency_count();
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
        tasks.len()
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

    fn task(&self) -> &dyn Task<S> {
        self.task.as_ref()
    }

    fn n_dependencies(&self) -> usize {
        self.n_dependencies
    }

    fn indices_of_dependent_tasks(&self) -> &[usize] {
        &self.indices_of_dependent_tasks
    }

    /// Increments the count of completed dependencies.
    ///
    /// # Returns
    /// An enum indicating whether the task has no
    /// uncompleted dependencies left and is thus
    /// ready for execution.
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

    fn reset_completed_dependency_count(&self) {
        self.completed_dependency_count.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use impact_thread::WorkerID;
    use parking_lot::Mutex;
    use std::{iter, thread, time::Duration};

    const EXEC_ALL: ExecutionTag = ExecutionTag::from_str("all");

    #[derive(Debug)]
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
                .iter()
                .map(|&(worker_id, _)| worker_id)
                .collect()
        }

        fn get_recorded_task_ids(&self) -> Vec<TaskID> {
            self.recorded_tasks
                .lock()
                .iter()
                .map(|&(_, task_id)| task_id)
                .collect()
        }

        fn record_task(&self, worker_id: WorkerID, task_id: TaskID) {
            self.recorded_tasks.lock().push((worker_id, task_id));
            thread::sleep(Duration::from_millis(1));
        }
    }

    macro_rules! create_task_type {
        (name = $task:ident, deps = [$($dep:ty),*]) => {
            #[derive(Debug)]
            struct $task;

            impl $task {
                const ID: TaskID = TaskID::from_str(stringify!($task));
                const EXEC_TAG: ExecutionTag = ExecutionTag::from_str(stringify!($task));
            }

            impl Task<Arc<TaskRecorder>> for $task
            {
                fn id(&self) -> TaskID {
                    Self::ID
                }

                fn depends_on(&self) -> &[TaskID] {
                    &[$(<$dep>::ID),*]
                }

                fn should_execute(&self, execution_tags: &ExecutionTags) -> bool {
                    [EXEC_ALL, Self::EXEC_TAG].iter().any(|tag| execution_tags.contains(tag))
                }

                fn execute(&self, _task_recorder: &Arc<TaskRecorder>) -> Result<()> {
                    unreachable!()
                }

                fn execute_with_worker(&self, worker_id: WorkerID, task_recorder: &Arc<TaskRecorder>) -> Result<()> {
                    Ok(task_recorder.record_task(worker_id, self.id()))
                }
            }
        };
        (name = $task:ident, deps = [$($dep:ty),*], fails) => {
            #[derive(Debug)]
            struct $task;

            impl $task {
                const ID: TaskID = TaskID::from_str(stringify!($task));
                const EXEC_TAG: ExecutionTag = ExecutionTag::from_str(stringify!($task));
            }

            impl Task<Arc<TaskRecorder>> for $task
            {
                fn id(&self) -> TaskID {
                    Self::ID
                }

                fn depends_on(&self) -> &[TaskID] {
                    &[$(<$dep>::ID),*]
                }

                fn should_execute(&self, execution_tags: &ExecutionTags) -> bool {
                    [EXEC_ALL, Self::EXEC_TAG].iter().any(|tag| execution_tags.contains(tag))
                }

                fn execute(&self, _task_recorder: &Arc<TaskRecorder>) -> Result<()> {
                    unreachable!()
                }

                fn execute_with_worker(&self, _worker_id: WorkerID, _task_recorder: &Arc<TaskRecorder>) -> Result<()> {
                    anyhow::bail!("{} always fails!", stringify!($task))
                }
            }
        };
        (name = $task:ident, deps = [$($dep:ty),*], skips) => {
            #[derive(Debug)]
            struct $task;

            impl $task {
                const ID: TaskID = TaskID::from_str(stringify!($task));
                #[allow(dead_code)]
                const EXEC_TAG: ExecutionTag = ExecutionTag::from_str(stringify!($task));
            }

            impl Task<Arc<TaskRecorder>> for $task
            {
                fn id(&self) -> TaskID {
                    Self::ID
                }

                fn depends_on(&self) -> &[TaskID] {
                    &[$(<$dep>::ID),*]
                }

                fn should_execute(&self, _execution_tags: &ExecutionTags) -> bool {
                    false // Always skip this task
                }

                fn execute(&self, _task_recorder: &Arc<TaskRecorder>) -> Result<()> {
                    unreachable!()
                }

                fn execute_with_worker(&self, _worker_id: WorkerID, _task_recorder: &Arc<TaskRecorder>) -> Result<()> {
                    unreachable!("This task should be skipped")
                }
            }
        };
    }

    create_task_type!(name = Task1, deps = []);
    create_task_type!(name = Task2, deps = []);
    create_task_type!(name = DepTask1, deps = [Task1]);
    create_task_type!(name = DepDepTask1, deps = [DepTask1]);
    create_task_type!(name = DepTask1Task2, deps = [Task1, Task2]);
    create_task_type!(name = DepDepTask1Task2, deps = [DepTask1, Task2]);
    create_task_type!(name = CircularTask1, deps = [CircularTask2]);
    create_task_type!(name = CircularTask2, deps = [CircularTask1]);

    create_task_type!(name = FailingTask, deps = [], fails);
    create_task_type!(name = DependentOnFailingTask, deps = [FailingTask]);

    type TestTaskScheduler = TaskScheduler<Arc<TaskRecorder>>;
    type TestTaskDependencyGraph = TaskDependencyGraph<Arc<TaskRecorder>>;
    type TestOrderedTask = OrderedTask<Arc<TaskRecorder>>;

    fn create_scheduler(n_workers: usize) -> TestTaskScheduler {
        TaskScheduler::new(
            NonZeroUsize::new(n_workers).unwrap(),
            NonZeroUsize::new(16).unwrap(),
            Arc::new(TaskRecorder::new()),
        )
    }

    #[test]
    fn registering_tasks_in_dependency_order_works() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(Task1).unwrap();
        assert!(scheduler.has_task(Task1));

        scheduler.register_task(Task2).unwrap();
        assert!(scheduler.has_task(Task1));
        assert!(scheduler.has_task(Task2));

        scheduler.register_task(DepTask1).unwrap();
        assert!(scheduler.has_task(Task1));
        assert!(scheduler.has_task(Task1));
        assert!(scheduler.has_task(DepTask1));

        scheduler.register_task(DepDepTask1Task2).unwrap();
        assert!(scheduler.has_task(Task1));
        assert!(scheduler.has_task(Task1));
        assert!(scheduler.has_task(DepTask1));
        assert!(scheduler.has_task(DepDepTask1Task2));

        scheduler.complete_task_registration().unwrap();
    }

    #[test]
    fn registering_tasks_out_of_dependency_order_works() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(DepDepTask1Task2).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2));

        scheduler.register_task(Task2).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2));
        assert!(scheduler.has_task(Task2));

        scheduler.register_task(DepTask1).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2));
        assert!(scheduler.has_task(Task2));
        assert!(scheduler.has_task(DepTask1));

        scheduler.register_task(Task1).unwrap();
        assert!(scheduler.has_task(DepDepTask1Task2));
        assert!(scheduler.has_task(Task2));
        assert!(scheduler.has_task(DepTask1));
        assert!(scheduler.has_task(Task1));

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
        scheduler.execute(&Arc::new(ExecutionTags::default()));
    }

    #[test]
    fn registering_task_invalidates_executor() {
        let mut scheduler = create_scheduler(1);
        assert!(scheduler.get_executor().is_none());

        scheduler.register_task(Task1).unwrap();
        scheduler.complete_task_registration().unwrap();
        assert!(scheduler.get_executor().is_some());

        scheduler.register_task(Task2).unwrap();
        assert!(scheduler.get_executor().is_none());
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

        scheduler
            .execute_and_wait(&Arc::new(ExecutionTags::from_iter([EXEC_ALL])))
            .unwrap();
        let recorded_worker_ids = scheduler.external_state().get_recorded_worker_ids();
        let recorded_task_ids = scheduler.external_state().get_recorded_task_ids();

        // Verify that all tasks were executed
        assert_eq!(recorded_task_ids.len(), 5);
        assert!(recorded_task_ids.contains(&Task1::ID));
        assert!(recorded_task_ids.contains(&Task2::ID));
        assert!(recorded_task_ids.contains(&DepTask1::ID));
        assert!(recorded_task_ids.contains(&DepTask1Task2::ID));
        assert!(recorded_task_ids.contains(&DepDepTask1Task2::ID));

        // Verify that dependency constraints are respected
        let task1_pos = recorded_task_ids
            .iter()
            .position(|&id| id == Task1::ID)
            .unwrap();
        let task2_pos = recorded_task_ids
            .iter()
            .position(|&id| id == Task2::ID)
            .unwrap();
        let dep_task1_pos = recorded_task_ids
            .iter()
            .position(|&id| id == DepTask1::ID)
            .unwrap();
        let dep_task1_task2_pos = recorded_task_ids
            .iter()
            .position(|&id| id == DepTask1Task2::ID)
            .unwrap();
        let dep_dep_task1_task2_pos = recorded_task_ids
            .iter()
            .position(|&id| id == DepDepTask1Task2::ID)
            .unwrap();

        // DepTask1 must execute after Task1
        assert!(
            dep_task1_pos > task1_pos,
            "DepTask1 should execute after Task1"
        );

        // DepTask1Task2 must execute after both Task1 and Task2
        assert!(
            dep_task1_task2_pos > task1_pos,
            "DepTask1Task2 should execute after Task1"
        );
        assert!(
            dep_task1_task2_pos > task2_pos,
            "DepTask1Task2 should execute after Task2"
        );

        // DepDepTask1Task2 must execute after both DepTask1 and Task2
        assert!(
            dep_dep_task1_task2_pos > dep_task1_pos,
            "DepDepTask1Task2 should execute after DepTask1"
        );
        assert!(
            dep_dep_task1_task2_pos > task2_pos,
            "DepDepTask1Task2 should execute after Task2"
        );

        // Verify that both workers were used (load balancing)
        let unique_workers: HashSet<_> = recorded_worker_ids.iter().collect();
        assert!(
            unique_workers.len() > 1,
            "Expected multiple workers to be used for load balancing"
        );
    }

    #[test]
    fn filtering_execution_with_tags_works() {
        let mut scheduler = create_scheduler(2);
        scheduler.register_task(DepDepTask1Task2).unwrap();
        scheduler.register_task(Task2).unwrap();
        scheduler.register_task(DepTask1).unwrap();
        scheduler.register_task(Task1).unwrap();
        scheduler.register_task(DepTask1Task2).unwrap();
        scheduler.complete_task_registration().unwrap();

        scheduler
            .execute_and_wait(&Arc::new(ExecutionTags::from_iter([
                Task2::EXEC_TAG,
                DepTask1::EXEC_TAG,
                DepDepTask1Task2::EXEC_TAG,
            ])))
            .unwrap();
        let recorded_task_ids = scheduler.external_state().get_recorded_task_ids();

        for task_id in [Task2::ID, DepTask1::ID, DepDepTask1Task2::ID] {
            assert!(recorded_task_ids.contains(&task_id));
        }
        for task_id in [Task1::ID, DepTask1Task2::ID] {
            assert!(!recorded_task_ids.contains(&task_id));
        }
    }

    #[test]
    fn ordered_tasks_are_created_correctly() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(DepDepTask1).unwrap();
        scheduler.register_task(Task1).unwrap();
        scheduler.register_task(DepTask1).unwrap();
        scheduler.complete_task_registration().unwrap();

        let task_ordering = scheduler.get_executor().unwrap().task_ordering();
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
                .get_executor()
                .unwrap()
                .task_ordering()
                .n_dependencyless_tasks(),
            1
        );

        scheduler.register_task(Task2).unwrap();
        scheduler.register_task(DepDepTask1Task2).unwrap();
        scheduler.complete_task_registration().unwrap();
        assert_eq!(
            scheduler
                .get_executor()
                .unwrap()
                .task_ordering()
                .n_dependencyless_tasks(),
            2
        );
    }

    #[test]
    fn tasks_are_ordered_correctly() {
        let mut dependency_graph = TestTaskDependencyGraph::new();
        dependency_graph.add_task(&DepDepTask1Task2);
        dependency_graph.add_task(&Task1);
        dependency_graph.add_task(&DepTask1);
        dependency_graph.add_task(&DepTask1Task2);
        dependency_graph.add_task(&Task2);

        let ordered_task_ids = dependency_graph.obtain_ordered_task_ids().unwrap();

        match ordered_task_ids[..] {
            [
                Task1::ID,
                Task2::ID,
                DepTask1::ID,
                DepDepTask1Task2::ID,
                DepTask1Task2::ID,
            ]
            | [
                Task2::ID,
                Task1::ID,
                DepTask1::ID,
                DepDepTask1Task2::ID,
                DepTask1Task2::ID,
            ]
            | [
                Task1::ID,
                Task2::ID,
                DepTask1::ID,
                DepTask1Task2::ID,
                DepDepTask1Task2::ID,
            ]
            | [
                Task2::ID,
                Task1::ID,
                DepTask1::ID,
                DepTask1Task2::ID,
                DepDepTask1Task2::ID,
            ]
            | [
                Task1::ID,
                Task2::ID,
                DepTask1Task2::ID,
                DepTask1::ID,
                DepDepTask1Task2::ID,
            ]
            | [
                Task2::ID,
                Task1::ID,
                DepTask1Task2::ID,
                DepTask1::ID,
                DepDepTask1Task2::ID,
            ] => {}
            _ => panic!("Incorrect task order"),
        }
    }

    #[test]
    fn finding_dependent_task_ids_works() {
        let mut dependency_graph = TestTaskDependencyGraph::new();
        dependency_graph.add_task(&DepTask1);
        dependency_graph.add_task(&DepDepTask1Task2);
        dependency_graph.add_task(&Task1);
        dependency_graph.add_task(&DepTask1Task2);
        dependency_graph.add_task(&Task2);
        dependency_graph.add_task(&DepDepTask1);

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

    #[test]
    fn failing_single_task_completes_with_error() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(FailingTask).unwrap();
        scheduler.complete_task_registration().unwrap();

        let result = scheduler.execute_and_wait(&Arc::new(ExecutionTags::from_iter([EXEC_ALL])));

        let errors = result.unwrap_err();
        assert_eq!(errors.n_errors(), 1);
        assert!(errors.get_error_of(FailingTask::ID).is_some());
    }

    #[test]
    fn failing_task_with_single_dependent_completes_with_error() {
        let mut scheduler = create_scheduler(1);
        scheduler.register_task(FailingTask).unwrap();
        scheduler.register_task(DependentOnFailingTask).unwrap();
        scheduler.complete_task_registration().unwrap();

        let result = scheduler.execute_and_wait(&Arc::new(ExecutionTags::from_iter([EXEC_ALL])));

        let errors = result.unwrap_err();
        assert_eq!(errors.n_errors(), 1);
        assert!(errors.get_error_of(FailingTask::ID).is_some());

        // Verify dependent task was never recorded as executed
        let task_recorder = scheduler.external_state();
        let recorded_tasks = task_recorder.get_recorded_task_ids();
        assert!(!recorded_tasks.contains(&DependentOnFailingTask::ID));
    }

    #[test]
    fn mixed_success_and_failure_tasks_complete_correctly() {
        let mut scheduler = create_scheduler(2);
        scheduler.register_task(Task1).unwrap(); // This should succeed
        scheduler.register_task(FailingTask).unwrap(); // This should fail
        scheduler.register_task(DepTask1).unwrap(); // This depends on Task1 and should succeed
        scheduler.complete_task_registration().unwrap();

        let result = scheduler.execute_and_wait(&Arc::new(ExecutionTags::from_iter([EXEC_ALL])));

        let errors = result.unwrap_err();
        assert_eq!(errors.n_errors(), 1);
        assert!(errors.get_error_of(FailingTask::ID).is_some());

        // Successful tasks should have been executed
        let task_recorder = scheduler.external_state();
        let recorded_tasks = task_recorder.get_recorded_task_ids();
        assert!(recorded_tasks.contains(&Task1::ID));
        assert!(recorded_tasks.contains(&DepTask1::ID));
        assert!(!recorded_tasks.contains(&FailingTask::ID));
    }

    #[test]
    fn failing_task_in_dependency_chain_stops_execution() {
        // Create a dependency chain: Task1 -> FailingDependentTask -> FinalTask
        // When the middle task fails, the final task should not execute

        create_task_type!(name = FailingDependentTask, deps = [Task1], fails);
        create_task_type!(name = FinalTask, deps = [FailingDependentTask]);

        let mut scheduler = create_scheduler(1);
        scheduler.register_task(Task1).unwrap();
        scheduler.register_task(FailingDependentTask).unwrap();
        scheduler.register_task(FinalTask).unwrap();
        scheduler.complete_task_registration().unwrap();

        let result = scheduler.execute_and_wait(&Arc::new(ExecutionTags::from_iter([EXEC_ALL])));

        let errors = result.unwrap_err();
        assert_eq!(errors.n_errors(), 1);
        assert!(errors.get_error_of(FailingDependentTask::ID).is_some());

        let task_recorder = scheduler.external_state();
        let recorded_tasks = task_recorder.get_recorded_task_ids();
        assert!(recorded_tasks.contains(&Task1::ID));
        assert!(!recorded_tasks.contains(&FinalTask::ID));
    }

    #[test]
    fn multiple_failing_tasks_all_report_errors() {
        create_task_type!(name = FailingTask2, deps = [], fails);

        let mut scheduler = create_scheduler(2);
        scheduler.register_task(FailingTask).unwrap();
        scheduler.register_task(FailingTask2).unwrap();
        scheduler.complete_task_registration().unwrap();

        let result = scheduler.execute_and_wait(&Arc::new(ExecutionTags::from_iter([EXEC_ALL])));

        let errors = result.unwrap_err();
        assert_eq!(errors.n_errors(), 2);
        assert!(errors.get_error_of(FailingTask::ID).is_some());
        assert!(errors.get_error_of(FailingTask2::ID).is_some());
    }
}
