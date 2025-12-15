//! Thread pool implementation.

use crossbeam_channel::{Receiver, Sender, TrySendError};
use impact_profiling::instrumentation;
use parking_lot::{Condvar, Mutex};
use std::{
    fmt,
    num::NonZeroUsize,
    panic,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
};

/// A set of worker threads configured to execute a specific task on request.
///
/// # Examples
/// ```no_run
/// # use impact_thread::pool::{ThreadPool};
/// # use std::{iter, num::NonZeroUsize, sync::Arc};
/// # use parking_lot::Mutex;
/// #
/// let n_workers = 2;
/// let queue_capacity = 256;
/// let n_tasks = 2;
///
/// let pool = ThreadPool::new(
///     // At least one worker is required
///     NonZeroUsize::new(n_workers).unwrap(),
///     NonZeroUsize::new(queue_capacity).unwrap(),
///     // Define task closure that increments a shared count
///     &|_channel, (count, incr): (Arc<Mutex<usize>>, usize)| {
///         *count.lock() += incr;
///     }
/// );
///
/// // Create shared mutex with initial count
/// let count = Arc::new(Mutex::new(0));
/// // Amount to increment count
/// let incr = 3;
///
/// // Create one message for each task execution, each containing
/// // a reference to the shared count and the increment
/// let messages = iter::repeat_with(|| (Arc::clone(&count), incr)).take(n_tasks);
///
/// // Execute the tasks and wait until all tasks are completed
/// pool.execute_and_wait(messages).unwrap();
///
/// assert_eq!(*count.lock(), n_workers * incr);
/// ```
///
/// # Type parameters
/// `M` is the type of message content sent to threads when they should execute
/// a task.
#[derive(Debug)]
pub struct ThreadPool<M> {
    communicator: ThreadPoolCommunicator<M>,
    workers: Vec<Worker>,
}

/// A [`ThreadPool`] where the message type `M` is a boxed closure, enabling
/// tasks to be dynamically specified at execution time rather than at pool
/// construction time.
pub type DynamicThreadPool = ThreadPool<DynamicTask<'static>>;

/// A boxed closure representing a [`DynamicThreadPool`] task with lifetime
/// `'t`. Tasks executed directly on the `DynamicThreadPool` must have the
/// static lifetime, but shorter lifetimes are allowed for tasks executed
/// through a [`ThreadPoolScope`].
#[allow(missing_debug_implementations, clippy::type_complexity)]
pub struct DynamicTask<'t>(Box<dyn FnOnce(&ThreadPoolChannel<DynamicTask<'t>>) + Send + 't>);

/// A scope of execution for a [`DynamicThreadPool`], obtainable by calling
/// [`DynamicThreadPool::with_scope`]. Tasks executed through the scope can have
/// any lifetime longer than that of the scope.
#[allow(missing_debug_implementations)]
pub struct ThreadPoolScope<'s> {
    pool: &'s DynamicThreadPool,
}

pub type ThreadPoolResult = Result<(), ThreadPoolError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadPoolError {
    QueueFull,
    ChannelDisconnected,
    WorkerPanic,
    TaskScheduledDuringShutdown,
}

#[derive(Debug)]
struct Worker {
    handle: JoinHandle<()>,
}

/// A shared structure for handling communication between the threads in a
/// [`ThreadPool`].
#[derive(Debug)]
struct ThreadPoolCommunicator<M> {
    n_workers: NonZeroUsize,
    channel: ThreadPoolChannel<M>,
    execution_progress: Arc<ExecutionProgress>,
}

/// A single channel shared between the main thread and all worker threads in a
/// [`ThreadPool`], used for sending and receiving instructions to and from a
/// shared queue.
///
/// External task implementations can use this channel to initiate additional
/// task executions.
#[derive(Debug)]
pub struct ThreadPoolChannel<M> {
    owning_worker_id: Option<WorkerID>,
    sender: Sender<WorkerInstruction<M>>,
    receiver: Receiver<WorkerInstruction<M>>,
    execution_progress: Arc<ExecutionProgress>,
}

#[derive(Debug)]
struct ExecutionProgress {
    pending_task_count: AtomicUsize,
    wait_mutex: Mutex<()>,
    no_pending_tasks_condvar: Condvar,
    panic_count: AtomicUsize,
    is_shutting_down: AtomicBool,
}

/// An instruction that can be sent to threads in a [`ThreadPool`] to make them
/// begin executing their task with a given message of type `M` (which can be
/// any piece of data), or to terminate so that they can be joined.
#[derive(Clone, Debug, PartialEq, Eq)]
enum WorkerInstruction<M> {
    Execute(M),
    Terminate,
}

/// ID identifying worker threads in a [`ThreadPool`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WorkerID(u64);

impl<M> ThreadPool<M> {
    /// Creates a new thread pool containing the given number of worker threads
    /// configured to execute a specified task. When a thread receives a
    /// [`WorkerInstruction`] to execute the task, the given `execute_task`
    /// closure is called. The closure is supplied with the message contained in
    /// the execution instruction as well as a reference to a
    /// [`ThreadPoolChannel`] that can be used to send messages to other worker
    /// threads from the closure.
    pub fn new<T>(
        n_workers: NonZeroUsize,
        queue_capacity: NonZeroUsize,
        execute_task: &'static T,
    ) -> Self
    where
        M: Send + 'static,
        T: Fn(&ThreadPoolChannel<M>, M) + Sync,
    {
        let communicator = ThreadPoolCommunicator::new(n_workers, queue_capacity);

        let workers = (0..n_workers.get() as u64)
            .map(|worker_id| {
                // Create a new instance of the shared communicator for the
                // spawned worker to use
                let communicator = communicator.clone_for_worker(WorkerID(worker_id));

                Worker::spawn(communicator, execute_task)
            })
            .collect();

        Self {
            communicator,
            workers,
        }
    }

    /// Returns the number of worker threads in the thread pool (this does not
    /// include the main thread).
    pub fn n_workers(&self) -> NonZeroUsize {
        self.communicator.n_workers()
    }

    /// Instructs worker threads in the pool to execute their task. The task
    /// will be executed with each of the given messages. This function does not
    /// return until all the given task executions as well as all additional
    /// executions initiated by those tasks have been completed. To avoid
    /// blocking the calling thread, use [`execute`](Self::execute) instead.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The queue is full or has become disconnected.
    /// - The pool is currently shutting down.
    /// - Any task in the thread pool has panicked since the pool was created or
    ///   [`Self::reset_panics`] was called.
    pub fn execute_and_wait(&self, messages: impl IntoIterator<Item = M>) -> ThreadPoolResult {
        self.execute(messages)?;
        self.wait_until_done()
    }

    /// Instructs worker threads in the pool to execute their task. The task
    /// will be executed with each of the given messages. This function returns
    /// as soon as all the given execution instructions have been sent. To wait
    /// until all tasks (including the executions initiated by the given tasks
    /// executions) have been completed and obtain any errors produced by the
    /// executed tasks, call [`wait_until_done`](Self::wait_until_done).
    ///
    /// # Errors
    /// Returns an error if:
    /// - The queue is full or has become disconnected.
    /// - The pool is currently shutting down.
    pub fn execute(&self, messages: impl IntoIterator<Item = M>) -> ThreadPoolResult {
        for message in messages {
            self.communicator
                .channel()
                .send_execute_instruction(message)?;
        }
        Ok(())
    }

    /// Blocks the calling thread and returns as soon as all task executions
    /// have been performed.
    ///
    /// # Errors
    /// Returns an error if any task in the thread pool has panicked since the
    /// pool was created or [`Self::reset_panics`] was called.
    pub fn wait_until_done(&self) -> ThreadPoolResult {
        let execution_progress = self.communicator.execution_progress();

        execution_progress.wait_for_no_pending_tasks();

        if execution_progress.panic_count() > 0 {
            Err(ThreadPoolError::WorkerPanic)
        } else {
            Ok(())
        }
    }

    /// Forgets any panics registered for executed tasks.
    pub fn reset_panics(&self) {
        self.communicator.execution_progress().reset_panic_count();
    }
}

impl DynamicThreadPool {
    /// Creates a new [`DynamicThreadPool`] with the given number of workers and
    /// capacity for the communication channel.
    pub fn new_dynamic(n_workers: NonZeroUsize, queue_capacity: NonZeroUsize) -> Self {
        Self::new(n_workers, queue_capacity, &|channel, task| task.0(channel))
    }

    /// Calls the given closure with a scope that can be used for executing
    /// tasks that borrow values with non-static lifetimes as long as their
    /// lifetimes exceed that of the scope.
    ///
    /// Once the closure returns, the main thread is blocked until there are no
    /// more tasks to execute.
    ///
    /// # Examples
    /// ```no_run
    /// # use impact_thread::pool::{DynamicTask, ThreadPool};
    /// # use std::num::NonZeroUsize;
    /// #
    /// let n_workers = 2;
    /// let queue_capacity = 256;
    ///
    /// let pool = ThreadPool::new_dynamic(
    ///     NonZeroUsize::new(n_workers).unwrap(),
    ///     NonZeroUsize::new(queue_capacity).unwrap(),
    /// );
    ///
    /// let mut data = vec![0, 0, 0, 0];
    ///
    /// pool.with_scope(|scope| {
    ///     // The tasks borrow from the local `data` variable,
    ///     // which is okay because it outlives the scope
    ///     let tasks = data.iter_mut().map(|value| {
    ///         DynamicTask::new(|_| {
    ///             *value += 1;
    ///         })
    ///     });
    ///
    ///     scope.execute(tasks).unwrap();
    /// })
    /// .unwrap();
    ///
    /// // All values should have been incremented
    /// assert_eq!(data, vec![1, 1, 1, 1]);
    /// ```
    ///
    /// # Returns
    /// The return value of the closure, or an error if any task in the thread
    /// pool has panicked since the pool was created or [`Self::reset_panics`]
    /// was called.
    pub fn with_scope<'p, 's, F, R>(&'p self, f: F) -> Result<R, ThreadPoolError>
    where
        F: FnOnce(ThreadPoolScope<'s>) -> R,
        'p: 's,
    {
        // Create guard that will wait for no pending tasks when dropped
        struct ScopeGuard<'a>(&'a DynamicThreadPool);

        impl<'a> Drop for ScopeGuard<'a> {
            fn drop(&mut self) {
                self.0
                    .communicator
                    .execution_progress()
                    .wait_for_no_pending_tasks();
            }
        }

        let result = {
            // The guard will drop even if `f` panics, ensuring that we always wait
            // for completion
            let _guard = ScopeGuard(self);

            f(ThreadPoolScope::new(self))
        };

        if self.communicator.execution_progress().panic_count() > 0 {
            Err(ThreadPoolError::WorkerPanic)
        } else {
            Ok(result)
        }
    }
}

impl<M> Drop for ThreadPool<M> {
    fn drop(&mut self) {
        let execution_progress = self.communicator.execution_progress();

        // Prevent new tasks from being accepted while we shut down
        execution_progress.report_shutdown_started();

        // Make sure all tasks are completed before proceeding
        execution_progress.wait_for_no_pending_tasks();

        // Send a termination instruction for each of the workers once all tasks
        // are complete
        for _ in 0..self.workers.len() {
            let _ = self
                .communicator
                .channel()
                .try_send_instruction(WorkerInstruction::Terminate);
        }

        // Join each worker as soon as it has terminated
        for worker in self.workers.drain(..) {
            worker.join();
        }
    }
}

impl<'t> DynamicTask<'t> {
    /// Creates a new dynamic task represented by the given closure.
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(&ThreadPoolChannel<DynamicTask<'t>>) + Send + 't,
    {
        Self(Box::new(f))
    }
}

impl<'t> fmt::Debug for DynamicTask<'t> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicTask").finish()
    }
}

impl<'s> ThreadPoolScope<'s> {
    fn new(pool: &'s DynamicThreadPool) -> Self {
        Self { pool }
    }

    /// Instructs worker threads in the pool to execute the given tasks. This
    /// function returns as soon as all the tasks have been scheduled for
    /// execution. The tasks can have any lifetime as long as they outlive the
    /// scope.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The queue is full or has become disconnected.
    /// - The pool is currently shutting down.
    pub fn execute<'t>(&self, tasks: impl IntoIterator<Item = DynamicTask<'t>>) -> ThreadPoolResult
    where
        't: 's,
    {
        self.pool.execute(tasks.into_iter().map(|task|
            // SAFETY: The lifetime constraint in this method ensures that
            // values borrowed by the task will outlive the scope. Since the
            // only way to obtain a `ThreadPoolScope` is through
            // `DynamicThreadPool::with_scope`, which calls `wait_until_done` at
            // the end of the scope's lifetime, once that lifetime ends this
            // task must have been executed. Once a dynamic task has been
            // executed it will not be stored anywhere in memory (the pool only
            // stores pending tasks in the channel, and there is no way for a
            // task to receive another task from the channel and store it), so
            // any captured references in the task will never be dereferenced by
            // a worker thread again.
            unsafe { std::mem::transmute::<DynamicTask<'t>, DynamicTask<'static>>(task) }))
    }
}

impl fmt::Display for ThreadPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => write!(f, "Thread pool task queue is full")?,
            Self::ChannelDisconnected => write!(f, "Thread pool channel was disconnected")?,
            Self::WorkerPanic => write!(f, "One or more worker threads panicked")?,
            Self::TaskScheduledDuringShutdown => write!(
                f,
                "A task execution was requested while the pool was shutting down"
            )?,
        }
        Ok(())
    }
}

impl std::error::Error for ThreadPoolError {}

impl Worker {
    /// Spawns a new worker thread for executing the given task closure.
    fn spawn<M, F>(communicator: ThreadPoolCommunicator<M>, execute_tasks: &'static F) -> Self
    where
        M: Send + 'static,
        F: Fn(&ThreadPoolChannel<M>, M) + Sync,
    {
        let handle = thread::spawn(move || {
            let worker_id = communicator.channel().owning_worker_id();
            impact_log::trace!("Worker {worker_id} spawned");

            instrumentation::set_thread_name(&format!("Worker {worker_id}"));

            loop {
                let Some(instruction) = communicator.channel().wait_for_next_instruction() else {
                    // Channel disconnected
                    break;
                };

                match instruction {
                    WorkerInstruction::Execute(message) => {
                        if let Err(cause) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                            execute_tasks(communicator.channel(), message);
                        })) {
                            communicator.execution_progress().register_panic();
                            impact_log::error!("Task panicked: {cause:?}");
                        };

                        communicator
                            .execution_progress()
                            .register_completed_tasks(1);
                    }
                    WorkerInstruction::Terminate => {
                        impact_log::trace!("Worker {worker_id} terminating");
                        break;
                    }
                }
            }
        });
        Self { handle }
    }

    fn join(self) {
        if let Err(err) = self.handle.join() {
            impact_log::error!("Worker thread failed to join: {err:?}");
        }
    }
}

impl<M> ThreadPoolCommunicator<M> {
    fn new(n_workers: NonZeroUsize, queue_capacity: NonZeroUsize) -> Self {
        let execution_progress = Arc::new(ExecutionProgress::new());
        let channel = ThreadPoolChannel::new(queue_capacity, execution_progress.clone());
        Self {
            n_workers,
            channel,
            execution_progress,
        }
    }

    fn n_workers(&self) -> NonZeroUsize {
        self.n_workers
    }

    fn channel(&self) -> &ThreadPoolChannel<M> {
        &self.channel
    }

    fn execution_progress(&self) -> &ExecutionProgress {
        &self.execution_progress
    }

    /// Creates a new instance of the communicator for use by the given worker
    /// to communicate with the other threads in the [`ThreadPool`].
    fn clone_for_worker(&self, worker_id: WorkerID) -> Self {
        Self {
            n_workers: self.n_workers,
            channel: self.channel.clone_for_worker(worker_id),
            execution_progress: self.execution_progress.clone(),
        }
    }
}

impl<M> ThreadPoolChannel<M> {
    fn new(capacity: NonZeroUsize, execution_progress: Arc<ExecutionProgress>) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(capacity.get());
        Self {
            owning_worker_id: None,
            sender,
            receiver,
            execution_progress,
        }
    }

    /// Sends an instruction to execute the task with the given message to the
    /// receiving queue shared between the workers. The first available worker
    /// will execute the task once with the given message.
    ///
    /// # Errors
    /// Returns an error if the queue is full or has become disconnected, or if
    /// the pool is currently shutting down.
    pub fn send_execute_instruction(&self, message: M) -> ThreadPoolResult {
        self.execution_progress.add_to_pending_task_count(1);

        if self.execution_progress.is_shutting_down() {
            self.execution_progress.register_completed_tasks(1);
            return Err(ThreadPoolError::TaskScheduledDuringShutdown);
        }

        self.try_send_instruction(WorkerInstruction::Execute(message))
            .inspect_err(|_| {
                // If we could not send the message, the initial increment must
                // be undone
                self.execution_progress.register_completed_tasks(1);
            })
    }

    fn try_send_instruction(&self, instruction: WorkerInstruction<M>) -> ThreadPoolResult {
        self.sender.try_send(instruction).map_err(|err| match err {
            TrySendError::Full(_) => ThreadPoolError::QueueFull,
            TrySendError::Disconnected(_) => ThreadPoolError::ChannelDisconnected,
        })
    }

    /// Returns `None` if the channel was disconnected.
    fn wait_for_next_instruction(&self) -> Option<WorkerInstruction<M>> {
        self.receiver.recv().ok()
    }

    /// Returns the ID of the worker owning this instance of the
    /// [`ThreadPool`]'s channel.
    ///
    /// # Panics
    /// If called on a [`ThreadPoolChannel`] that has not been assigned to a
    /// worker thread.
    fn owning_worker_id(&self) -> WorkerID {
        self.owning_worker_id.unwrap()
    }

    /// Creates a new instance of the channel for use by the given worker to
    /// exchange instructions with the other threads in the [`ThreadPool`].
    fn clone_for_worker(&self, worker_id: WorkerID) -> Self {
        Self {
            owning_worker_id: Some(worker_id),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            execution_progress: self.execution_progress.clone(),
        }
    }
}

impl ExecutionProgress {
    fn new() -> Self {
        let pending_task_count = AtomicUsize::new(0);
        let wait_mutex = Mutex::default();
        let no_pending_tasks_condvar = Condvar::new();
        let panic_count = AtomicUsize::new(0);
        let is_shutting_down = AtomicBool::new(false);
        Self {
            pending_task_count,
            wait_mutex,
            no_pending_tasks_condvar,
            panic_count,
            is_shutting_down,
        }
    }

    /// Increments the atomic count of pending tasks by the given number.
    fn add_to_pending_task_count(&self, n_tasks: usize) {
        if n_tasks == 0 {
            return;
        }
        self.pending_task_count.fetch_add(n_tasks, Ordering::AcqRel);
    }

    /// Decrements the atomic count of pending tasks by the given number and
    /// updates the conditional variable used for tracking whether there are
    /// pending tasks.
    ///
    /// # Panics
    /// If the count is attempted to be decremented below zero.
    fn register_completed_tasks(&self, n_tasks: usize) {
        if n_tasks == 0 {
            return;
        }

        let previous_count = self.pending_task_count.fetch_sub(n_tasks, Ordering::AcqRel);
        assert!(
            previous_count >= n_tasks,
            "Underflow when registering executed tasks"
        );

        if previous_count == n_tasks {
            // We have gone from `n_tasks` pending tasks to zero pending tasks.
            // Workers waiting on the condition variable in
            // `wait_for_no_pending_tasks` must be notified so they can proceed.
            // We acquire a lock on the wait mutex first so that our notify
            // can't happen between the waiting worker checking that
            // `pending_task_count` is zero and it calling `wait` on the
            // condition variable. Otherwise we might call `notify_all` before
            // the waiter called `wait`, in which case the notification would be
            // lost.
            let _guard = self.wait_mutex.lock();
            self.no_pending_tasks_condvar.notify_all();
        }
    }

    /// Blocks execution in the calling thread and resumes when the count of
    /// pending tasks is zero.
    fn wait_for_no_pending_tasks(&self) {
        // Check if already zero
        if self.pending_task_count.load(Ordering::Acquire) == 0 {
            return;
        }

        // If not, we must wait until notified
        let mut guard = self.wait_mutex.lock();
        while self.pending_task_count.load(Ordering::Acquire) != 0 {
            self.no_pending_tasks_condvar.wait(&mut guard);
        }
    }

    #[cfg(test)]
    fn pending_task_count(&self) -> usize {
        self.pending_task_count.load(Ordering::Acquire)
    }

    fn report_shutdown_started(&self) {
        self.is_shutting_down.store(true, Ordering::Release);
    }

    fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::Acquire)
    }

    fn register_panic(&self) {
        self.panic_count.fetch_add(1, Ordering::AcqRel);
    }

    fn panic_count(&self) -> usize {
        self.panic_count.load(Ordering::Acquire)
    }

    fn reset_panic_count(&self) {
        self.panic_count.store(0, Ordering::Release);
    }
}

impl From<WorkerID> for u64 {
    fn from(id: WorkerID) -> Self {
        id.0
    }
}

impl fmt::Display for WorkerID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter;

    struct NoMessage;

    fn communicator<M>(n_workers: usize) -> ThreadPoolCommunicator<M> {
        ThreadPoolCommunicator::new(
            NonZeroUsize::new(n_workers).unwrap(),
            NonZeroUsize::new(16).unwrap(),
        )
    }

    fn thread_pool<M, T>(n_workers: usize, execute_task: &'static T) -> ThreadPool<M>
    where
        M: Send + 'static,
        T: Fn(&ThreadPoolChannel<M>, M) + Sync,
    {
        ThreadPool::new(
            NonZeroUsize::new(n_workers).unwrap(),
            NonZeroUsize::new(10).unwrap(),
            execute_task,
        )
    }

    #[test]
    fn creating_thread_communicator_works() {
        let n_workers = 2;
        let comm = communicator::<NoMessage>(n_workers);
        assert_eq!(comm.n_workers().get(), n_workers);
    }

    #[test]
    fn sending_message_with_communicator_works() {
        let n_workers = 1;
        let comm = communicator(n_workers);
        comm.channel().send_execute_instruction(42).unwrap();
        let message = comm.channel().wait_for_next_instruction().unwrap();
        assert_eq!(message, WorkerInstruction::Execute(42));
    }

    #[test]
    fn keeping_track_of_pending_task_count_works() {
        let n_workers = 1;
        let comm = communicator::<NoMessage>(n_workers);
        assert_eq!(comm.execution_progress().pending_task_count(), 0);
        comm.execution_progress().add_to_pending_task_count(2);
        assert_eq!(comm.execution_progress().pending_task_count(), 2);
        comm.execution_progress().add_to_pending_task_count(1);
        assert_eq!(comm.execution_progress().pending_task_count(), 3);

        comm.execution_progress().register_completed_tasks(2);
        assert_eq!(comm.execution_progress().pending_task_count(), 1);
        comm.execution_progress().register_completed_tasks(1);
        assert_eq!(comm.execution_progress().pending_task_count(), 0);

        comm.execution_progress().wait_for_no_pending_tasks(); // Should return
        // immediately
    }

    #[test]
    #[should_panic]
    fn registering_executed_task_when_none_are_pending_fails() {
        let n_workers = 2;
        let comm = communicator::<NoMessage>(n_workers);
        comm.execution_progress().register_completed_tasks(1);
    }

    #[test]
    fn creating_thread_pool_works() {
        let n_workers = 2;
        let pool = thread_pool::<NoMessage, _>(n_workers, &|_, _| {});
        assert_eq!(pool.n_workers().get(), n_workers);
    }

    #[test]
    fn executing_thread_pool_works() {
        let n_workers = 2;
        let count = Arc::new(Mutex::new(0));
        let pool = thread_pool(
            n_workers,
            &|_, (count, incr): (Arc<Mutex<usize>>, usize)| {
                *count.lock() += incr;
            },
        );
        pool.execute_and_wait(iter::repeat_with(|| (Arc::clone(&count), 3)).take(n_workers))
            .unwrap();
        drop(pool);
        assert_eq!(*count.lock(), n_workers * 3);
    }

    #[test]
    fn queue_full_error_when_queue_capacity_exceeded() {
        let n_workers = 1;
        let queue_capacity = 2;

        let (sx, rx) = crossbeam_channel::unbounded();

        let pool = ThreadPool::new(
            NonZeroUsize::new(n_workers).unwrap(),
            NonZeroUsize::new(queue_capacity).unwrap(),
            &|_, rx: Receiver<()>| {
                // Wait for message
                rx.recv().unwrap();
            },
        );

        // Fill queue beyond capacity
        let result = pool.execute((0..queue_capacity + 2).map(|_| rx.clone()));

        // Send messages so that the tasks can complete
        for _ in 0..queue_capacity {
            sx.send(()).unwrap();
        }

        assert_eq!(result, Err(ThreadPoolError::QueueFull));
    }

    #[test]
    fn worker_panic_is_caught_and_reported() {
        let n_workers = 1;
        let pool = thread_pool(n_workers, &|_, _: ()| {
            panic!("Intentional panic for testing");
        });

        let result = pool.execute_and_wait(std::iter::once(()));

        assert_eq!(result, Err(ThreadPoolError::WorkerPanic));
    }

    #[test]
    fn dynamic_thread_pool_works() {
        let n_workers = 2;
        let counter = Arc::new(Mutex::new(0));
        let pool = DynamicThreadPool::new_dynamic(
            NonZeroUsize::new(n_workers).unwrap(),
            NonZeroUsize::new(10).unwrap(),
        );

        let tasks = (0..n_workers).map(|_| {
            let counter = Arc::clone(&counter);
            DynamicTask::new(move |_| {
                *counter.lock() += 1;
            })
        });

        pool.execute_and_wait(tasks).unwrap();
        assert_eq!(*counter.lock(), n_workers);
    }

    #[test]
    fn nested_task_execution_works() {
        let n_workers = 1;
        let counter = Arc::new(Mutex::new(0));
        let pool = thread_pool(n_workers, &|channel,
                                            (counter, should_spawn): (
            Arc<Mutex<usize>>,
            bool,
        )| {
            *counter.lock() += 1;
            if should_spawn {
                // Spawn another task from within a task
                channel.send_execute_instruction((counter, false)).unwrap();
            }
        });

        pool.execute_and_wait(std::iter::once((Arc::clone(&counter), true)))
            .unwrap();
        assert_eq!(*counter.lock(), 2); // Original task + nested task
    }

    #[test]
    fn scoped_execution_with_borrowed_data_works() {
        let n_workers = 2;
        let pool = ThreadPool::new_dynamic(
            NonZeroUsize::new(n_workers).unwrap(),
            NonZeroUsize::new(10).unwrap(),
        );

        let mut data = vec![0, 0, 0, 0];

        pool.with_scope(|scope| {
            let tasks = data.iter_mut().map(|value| {
                DynamicTask::new(|_| {
                    *value += 1;
                })
            });

            scope.execute(tasks).unwrap();
        })
        .unwrap();

        // All values should have been incremented
        assert_eq!(data, vec![1, 1, 1, 1]);
    }

    #[test]
    fn scoped_execution_with_nested_tasks_works() {
        let n_workers = 2;
        let pool = ThreadPool::new_dynamic(
            NonZeroUsize::new(n_workers).unwrap(),
            NonZeroUsize::new(10).unwrap(),
        );

        let mut vec1 = Vec::new();
        let mut vec2 = Vec::new();

        pool.with_scope(|scope| {
            // First task spawns a nested task
            let task = DynamicTask::new(|channel| {
                vec1.push(0);

                // Spawn a nested task
                let nested_task = DynamicTask::new(|_| {
                    vec2.push(1);
                });

                channel.send_execute_instruction(nested_task).unwrap();
            });

            scope.execute(std::iter::once(task)).unwrap();
        })
        .unwrap();

        // First counter incremented by main task, second by nested task
        assert_eq!(vec1, [0]);
        assert_eq!(vec2, [1]);
    }
}
