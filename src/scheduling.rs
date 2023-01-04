//! Task scheduling.

use crate::{
    hash::ConstStringHash,
    thread::{
        TaskClosureReturnValue, TaskError, TaskID, ThreadPool, ThreadPoolChannel, ThreadPoolResult,
    },
};
use anyhow::{anyhow, bail, Result};
use petgraph::{
    algo::{self, DfsSpace},
    graphmap::DiGraphMap,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[cfg(test)]
use crate::thread::WorkerID;

/// Represents a piece of work to be performed by a
/// worker thread in a [`TaskScheduler`].
///
/// # Type parameters
/// `S` is the type of an object representing the state
/// of the world that the task can modify.
pub trait Task<S>: Sync + Send + Debug {
    /// Returns a unique ID identifying this task.
    /// This could be generated from a task name
    /// by calling [`task_name_to_id`].
    fn id(&self) -> TaskID;

    /// Returns the ID of every other task that must
    /// have been completed before this task can be
    /// executed.
    fn depends_on(&self) -> &[TaskID];

    /// Executes the task and modifies the given world
    /// state accordingly. This method may fail and return
    /// an error.
    fn execute(&self, world_state: &S) -> Result<(), TaskError>;

    /// Whether this task should be included in a
    /// [`TaskScheduler`] execution tagged with the given
    /// tags.
    fn should_execute(&self, execution_tags: &ExecutionTags) -> bool;

    /// Like [`execute`](Self::execute), but the ID of the
    /// worker executing the task is included as an argument.
    /// Useful for testing.
    #[cfg(test)]
    fn execute_with_worker(&self, _worker_id: WorkerID, world_state: &S) -> Result<()> {
        self.execute(world_state)
    }
}

/// A task manager that can schedule execution of multiple
/// interdependent tasks.
#[derive(Debug)]
pub struct TaskScheduler<S> {
    n_workers: NonZeroUsize,
    tasks: TaskPool<S>,
    dependency_graph: TaskDependencyGraph<S>,
    executor: Option<TaskExecutor<S>>,
    world_state: Arc<S>,
}

/// A tag associated with an execution of a [`TaskScheduler`].
pub type ExecutionTag = ConstStringHash;

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

/// Macro for defining a new empty type that implements the
/// [`Task`] trait.
///
/// The macro takes as input the name of the new task type,
/// the other tasks (also defined with this macro) this task
/// depends on, the execution tags (defined with the
/// [`define_execution_tag`] macro) that should trigger this
/// task, and a closure that takes a reference to some world
/// object and executes the task on it.
///
/// # Examples
/// ```no_run
/// # use impact::{define_execution_tag, define_task, scheduling::TaskScheduler};
/// # use std::{num::NonZeroUsize, sync::Arc};
/// #
/// # struct World;
/// #
/// # impl World {
/// #     fn new() -> Self {Self}
/// #     fn compute_forces(&self) {}
/// #     fn update_trajectories(&self) {}
/// # }
/// #
/// define_task!(
///     /// This optional doc comment will be applied to the type.
///     /// Additional attributes (like `#[derive(..)]`) can also be added.
///     /// The optional `[pub]` in front of the name makes the type public.
///     #[derive(PartialEq)]
///     [pub] UpdateTrajectories,
///     // Array of tasks this task depends on
///     depends_on = [ComputeForces],
///     // Include this task in executions tagged with any of these tags
///     execute_on = [Physics],
///     // Closure executing the task, modifying the input object
///     |world: &World| {
///         world.update_trajectories();
///         // The closure must return a `Result<(), TaskError>`
///         Ok(())
///     }
/// );
///
/// // Define the task that the above task depends on
/// define_task!(
///     ComputeForces,
///     depends_on = [],
///     execute_on = [Physics],
///     |world: &World| {
///         world.compute_forces();
///         Ok(())
///     }
/// );
///
/// // Define the tag that will trigger execution of the tasks
/// define_execution_tag!(Physics);
///
/// let world = Arc::new(World::new());
/// let n_workers = NonZeroUsize::new(2).unwrap();
///
/// let mut scheduler = TaskScheduler::new(n_workers, world);
///
/// // Add newly defined tasks to scheduler
/// scheduler.register_task(ComputeForces).unwrap();
/// scheduler.register_task(UpdateTrajectories).unwrap();
/// scheduler.complete_task_registration().unwrap();
/// ```
#[macro_export]
macro_rules! define_task {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident,
        depends_on = [$($dep:ident),*],
        execute_on = [$($tag:ident),*],
        |$world:ident: &$world_ty:ty| $execute:expr
    ) => {
        $(#[$attributes])*
        #[derive(Copy, Clone, Debug)]
        $($pub)? struct $name;

        impl $name {
            $($pub)? const TASK_ID: $crate::thread::TaskID = $crate::hash::ConstStringHash::new(stringify!($name));

            const N_DEPENDENCIES: usize = $crate::count_ident_args!($($dep),*);
            const DEPENDENCY_IDS: [$crate::thread::TaskID; Self::N_DEPENDENCIES] = [$($dep::TASK_ID),*];

            const N_EXECUTION_TAGS: usize = $crate::count_ident_args!($($tag),*);
            const EXECUTION_TAGS: [$crate::scheduling::ExecutionTag; Self::N_EXECUTION_TAGS] = [$($tag::EXECUTION_TAG),*];
        }

        impl $crate::scheduling::Task<$world_ty> for $name {
            fn id(&self) -> $crate::thread::TaskID {
                Self::TASK_ID
            }

            fn depends_on(&self) -> &[$crate::thread::TaskID] {
                &Self::DEPENDENCY_IDS
            }

            fn execute(&self, $world: &$world_ty) -> anyhow::Result<()> {
                $execute
            }

            fn should_execute(&self, execution_tags: &$crate::scheduling::ExecutionTags) -> bool {
                Self::EXECUTION_TAGS.iter().any(|tag| execution_tags.contains(tag))
            }
        }
    };
}

/// Macro for defining a new empty type representing an
/// [`ExecutionTag`], for use in the [`define_task`] macro.
#[macro_export]
macro_rules! define_execution_tag {
    (
        $(#[$attributes:meta])*
        $([$pub:ident])? $name:ident
    ) => {
        $(#[$attributes])*
        #[derive(Copy, Clone, Debug)]
        $($pub)? struct $name;

        impl $name {
            $($pub)? const EXECUTION_TAG: $crate::scheduling::ExecutionTag = $crate::hash::ConstStringHash::new(stringify!($name));
        }
    };
}

/// Macro that creates a [`lazy_static`] [`Arc<ExecutionTags>`]
/// variable with the given name containing the given list of
/// execution tags (defined with the [`define_execution_tag`]
/// macro).
#[macro_export]
macro_rules! define_execution_tag_set {
    (
        $([$pub:ident])? $name:ident, [$($tag:ident),*]
    ) => {
        lazy_static::lazy_static! {
            $($pub)? static ref $name: ::std::sync::Arc<$crate::scheduling::ExecutionTags> = ::std::sync::Arc::new($crate::scheduling::ExecutionTags::from([$($tag::EXECUTION_TAG),*]));
        }
    };
}

impl<S> TaskScheduler<S>
where
    S: Sync + Send + 'static,
{
    /// Creates a new task scheduler that will operate with the
    /// given number of worker threads on the given world state.
    pub fn new(n_workers: NonZeroUsize, world_state: Arc<S>) -> Self {
        Self {
            n_workers,
            tasks: HashMap::new(),
            dependency_graph: TaskDependencyGraph::new(),
            executor: None,
            world_state,
        }
    }

    /// Returns the number of worker threads that will be used to
    /// execute tasks.
    pub fn n_workers(&self) -> usize {
        self.n_workers.get()
    }

    /// Returns the state of the world that the tasks can modify.
    pub fn world_state(&self) -> &S {
        self.world_state.as_ref()
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
            &self.tasks,
            &mut self.dependency_graph,
            Arc::clone(&self.world_state),
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
    /// A [`ThreadPoolTaskErrors`](crate::thread::ThreadPoolTaskErrors) containing
    /// the [`TaskError`](crate::thread::TaskError) of each failed task is
    /// returned if any of the executed tasks failed.
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
    /// A [`ThreadPoolTaskErrors`](crate::thread::ThreadPoolTaskErrors) containing
    /// the [`TaskError`](crate::thread::TaskError) of each failed task is
    /// returned if any of the executed tasks failed.
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

    #[allow(dead_code)]
    fn get_executor(&self) -> Option<&TaskExecutor<S>> {
        self.executor.as_ref()
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
                panic!(
                    "Task {} depends on same task ({}) multiple times",
                    task_id, dependence_task_id
                );
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
        let thread_pool = ThreadPool::new(n_workers, &Self::execute_task_and_schedule_dependencies);
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
                task.execute(self.state.world_state()).expect("Task failed");
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
            self.task_ordering().n_tasks(),
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

        log::debug!(
            "Worker {} obtained task {}",
            channel.owning_worker_id(),
            task.id()
        );

        // Execute the task only if it thinks it should be based on
        // the current execution tags
        if task.should_execute(execution_tags.as_ref()) {
            with_debug_logging!("Worker {} executing task {}",
                channel.owning_worker_id(),
                task.id();
                {
                    let result = { cfg_if::cfg_if! {
                        if #[cfg(test)] {
                            task.execute_with_worker(channel.owning_worker_id(), state.world_state())
                        } else {
                            task.execute(state.world_state())
                        }
                    }};

                    if let Err(error) = result {
                        // Return immediately with the task ID and an error
                        // if the task execution failed
                        return TaskClosureReturnValue::failure(task.id(), error);
                    }
                }
            );
        } else {
            log::debug!(
                "Worker {} skipped execution of task {}",
                channel.owning_worker_id(),
                task.id()
            );
        }

        // Find each of the tasks that depend on this one, and
        // increment its count of completed dependencies. We keep
        // track of any dependent tasks that have no uncompleted
        // dependencies left as a result of completing this task.
        let ready_dependent_task_indices: Vec<_> = ordered_task
            .indices_of_dependent_tasks()
            .iter()
            .cloned()
            .filter(|&dependent_task_idx| {
                let dependent_task = state.task_ordering().task(dependent_task_idx);
                let task_ready = dependent_task.complete_dependency();
                task_ready == TaskReady::Yes
            })
            .collect();

        // Schedule each task that has no dependencies left for
        // execution, leaving one for this thread to start executing
        // immediately
        if ready_dependent_task_indices.len() > 1 {
            for &ready_dependent_task_idx in &ready_dependent_task_indices[1..] {
                with_debug_logging!(
                        "Worker {} scheduling execution of task {}",
                        channel.owning_worker_id(),
                        state
                            .task_ordering()
                            .task(ready_dependent_task_idx)
                            .task()
                            .id();
                    channel.send_execute_instruction(Self::create_message(
                        &state,
                        &execution_tags,
                        ready_dependent_task_idx,
                    ))
                );
            }
        }
        if let Some(&ready_dependent_task_idx) = ready_dependent_task_indices.first() {
            Self::execute_task_and_schedule_dependencies(
                channel,
                (state, execution_tags, ready_dependent_task_idx),
            )
            // Increment executed task count returned from dependent
            // task to account for this task
            .with_incremented_task_count()
        } else {
            TaskClosureReturnValue::success()
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

    fn n_tasks(&self) -> usize {
        self.tasks.len()
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

        log::debug!(
            "Ordered tasks: {:?}",
            ordered_task_ids
                .iter()
                .map(|task_id| task_pool[task_id].id())
                .collect::<Vec<_>>()
        );

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
mod test {
    use super::*;
    use crate::hash::ConstStringHash;
    use std::{iter, sync::Mutex, thread, time::Duration};

    const EXEC_ALL: ExecutionTag = ExecutionTag::new("all");

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
        (name = $task:ident, deps = [$($dep:ty),*]) => {
            #[derive(Debug)]
            struct $task;

            impl $task {
                const ID: TaskID = ConstStringHash::new(stringify!($task));
                #[allow(dead_code)]
                const EXEC_TAG: ExecutionTag = ConstStringHash::new(stringify!($task));
            }

            impl Task<TaskRecorder> for $task
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
        scheduler.execute(&Arc::new(ExecutionTags::new()));
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
            .execute_and_wait(&Arc::new(ExecutionTags::from([EXEC_ALL])))
            .unwrap();
        let recorded_worker_ids = scheduler.world_state().get_recorded_worker_ids();
        let recorded_task_ids = scheduler.world_state().get_recorded_task_ids();

        match recorded_task_ids[..] {
            [Task1::ID, Task2::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID]
            | [Task2::ID, Task1::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID]
            | [Task1::ID, Task2::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID]
            | [Task2::ID, Task1::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID]
            | [Task1::ID, Task2::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID]
            | [Task2::ID, Task1::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID] => {}
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
            [WorkerID(0), WorkerID(1), WorkerID(0), WorkerID(1), WorkerID(0)]
            | [WorkerID(1), WorkerID(0), WorkerID(1), WorkerID(0), WorkerID(1)] => {}
            _ => panic!("Incorrect worker contribution"),
        }
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
            .execute_and_wait(&Arc::new(ExecutionTags::from([
                Task2::EXEC_TAG,
                DepTask1::EXEC_TAG,
                DepDepTask1Task2::EXEC_TAG,
            ])))
            .unwrap();
        let recorded_task_ids = scheduler.world_state().get_recorded_task_ids();

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
            [Task1::ID, Task2::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID]
            | [Task2::ID, Task1::ID, DepTask1::ID, DepDepTask1Task2::ID, DepTask1Task2::ID]
            | [Task1::ID, Task2::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID]
            | [Task2::ID, Task1::ID, DepTask1::ID, DepTask1Task2::ID, DepDepTask1Task2::ID]
            | [Task1::ID, Task2::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID]
            | [Task2::ID, Task1::ID, DepTask1Task2::ID, DepTask1::ID, DepDepTask1Task2::ID] => {}
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
}
