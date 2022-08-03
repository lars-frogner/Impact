//! Utilities for multithreading.

use anyhow::Error;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
};

/// A set of worker threads configured to execute a
/// specific task on request.
///
/// The threads can perform simple communication by
/// sending messages to a shared recieving queue.
///
/// # Examples
/// ```no_run
/// # use impact::thread::{ThreadPool, TaskClosureReturnValue};
/// # use std::{iter, num::NonZeroUsize, sync::{Arc, Mutex}};
/// #
/// let n_workers = 2;
/// let n_tasks = 2;
///
/// let pool = ThreadPool::new(
///     // At least one worker is required
///     NonZeroUsize::new(n_workers).unwrap(),
///     // Define task closure that increments a shared count
///     &|_channel, (count, incr): (Arc<Mutex<usize>>, usize)| {
///         *count.lock().unwrap() += incr;
///         // The closure must return a `TaskClosureReturnValue`
///         TaskClosureReturnValue::success()
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
/// // Execute the tasks and wait until all `n_tasks` are completed
/// pool.execute_and_wait(messages, n_tasks).unwrap();
///
/// assert_eq!(*count.lock().unwrap(), n_workers * incr);
/// ```
///
/// # Type parameters
/// `M` is the type of message content sent to threads
/// when they should execute a task.
#[derive(Debug)]
pub struct ThreadPool<M> {
    communicator: ThreadPoolCommunicator<M>,
    workers: Vec<Worker>,
}

/// An instruction that can be sent to threads in a [`ThreadPool`]
/// to make them begin executing their task with a given
/// messsage of type `M` (which can be any piece of data), or to
/// terminate so that they can be joined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorkerInstruction<M> {
    Execute(M),
    Terminate,
}

/// The type if ID used for worker threads in a [`ThreadPool`].
pub type WorkerID = usize;

/// Type of ID used for identifying tasks that can be performed
/// by worker threads in a [`ThreadPool`].
pub type TaskID = u64;

/// [`Result`] produced by the task closure executed by worker
/// threads in a [`ThreadPool`]. The [`Err`] variant contains
/// the [`TaskID`] of the failed task together with the
/// resulting [`TaskError`].
pub type TaskClosureResult = Result<(), (TaskID, TaskError)>;

/// Type of error produced by failed task executions in a
/// [`ThreadPool`].
pub type TaskError = Error;

/// The information returned from the task closure executed
/// by worker threads in a [`ThreadPool`].
#[derive(Debug)]
pub struct TaskClosureReturnValue {
    /// The total number of tasks executed by the closure call.
    pub n_executed_tasks: usize,
    /// The result of the task closure execution, which should be an
    /// [`Err`] if any of the tasks executed in the closure failed.
    pub result: TaskClosureResult,
}

/// [`Result`] returned by execution of a set of tasks
/// in a [`ThreadPool`].
pub type ThreadPoolResult = Result<(), ThreadPoolTaskErrors>;

/// Container for a non-empty set of [`TaskError`]s produced
/// by execution of a set of tasks in a [`ThreadPool`].
/// The errors can be looked up by [`TaskID`].
#[derive(Debug)]
pub struct ThreadPoolTaskErrors {
    errors: HashMap<TaskID, TaskError>,
}

/// A single channel shared between the main thread and all
/// worker threads in a [`ThreadPool`], used for sending and
/// recieving instructions to and from a shared queue.
#[derive(Debug)]
pub struct ThreadPoolChannel<M> {
    owning_worker_id: Option<WorkerID>,
    sender: Sender<WorkerInstruction<M>>,
    receiver: Arc<Mutex<Receiver<WorkerInstruction<M>>>>,
}

/// A shared structure for handling communication between
/// the threads in a [`ThreadPool`].
#[derive(Debug)]
struct ThreadPoolCommunicator<M> {
    n_workers: NonZeroUsize,
    channel: ThreadPoolChannel<M>,
    execution_progress: ExecutionProgress,
    task_status: TaskStatus,
}

#[derive(Clone, Debug)]
struct ExecutionProgress {
    pending_task_count: Arc<AtomicUsize>,
    no_pending_tasks_condvar: Arc<(Mutex<bool>, Condvar)>,
}

#[derive(Clone, Debug)]
struct TaskStatus {
    some_task_failed: Arc<AtomicBool>,
    errors_of_failed_tasks: Arc<Mutex<HashMap<TaskID, TaskError>>>,
}

#[derive(Debug)]
struct Worker {
    handle: JoinHandle<()>,
}

impl<M> ThreadPool<M> {
    /// Creates a new thread pool containing the given number
    /// of worker threads configured to execute a specified task.
    /// When a thread recieves a [`WorkerInstruction`] to execute
    /// the task, the given `execute_task` closure is called.
    /// The closure is supplied with the message contained in
    /// the execution instruction as well as a reference to a
    /// [`ThreadPoolChannel`] that can be used to send messages
    /// to other worker threads from the closure.
    pub fn new<T>(n_workers: NonZeroUsize, execute_task: &'static T) -> Self
    where
        M: Send + 'static,
        T: Fn(&ThreadPoolChannel<M>, M) -> TaskClosureReturnValue + Sync,
    {
        let communicator = ThreadPoolCommunicator::new(n_workers);

        let workers = (0..n_workers.get())
            .into_iter()
            .map(|worker_id| {
                // Create a new instance of the shared communicator
                // for the spawned worker to use
                let communicator = communicator.copy_for_worker(worker_id);

                Worker::spawn(communicator, execute_task)
            })
            .collect();

        Self {
            communicator,
            workers,
        }
    }

    /// Returns the number of worker threads in the thread pool
    /// (this does not include the main thread).
    pub fn n_workers(&self) -> NonZeroUsize {
        self.communicator.n_workers()
    }

    /// Instructs worker threads in the pool to execute their task.
    /// The task will be executed with each of the given messages.
    /// The `n_tasks` argument is the total number of task executions
    /// that will result from this function call (including executions
    /// initiated from within the task). This function does not return
    /// until all expected task executions have been performed. To
    /// avoid blocking the calling thread, use [`execute`](Self::execute)
    /// instead.
    ///
    /// # Errors
    /// A [`ThreadPoolTaskErrors`] containing the [`TaskError`] of each
    /// failed task is returned if any of the executed tasks failed.
    pub fn execute_and_wait(
        &self,
        messages: impl Iterator<Item = M>,
        n_tasks: usize,
    ) -> ThreadPoolResult {
        self.execute(messages, n_tasks);
        self.wait_until_done()
    }

    /// Instructs worker threads in the pool to execute their task.
    /// The task will be executed with each of the given messages.
    /// The `n_tasks` argument is the total number of task executions
    /// that will result from this function call (including executions
    /// initiated from within the task). This function returns as soon
    /// as all the given execution instructions have been sent. To
    /// wait until all tasks have been completed and obtain any errors
    /// produced by the executed tasks, call
    /// [`wait_until_done`](Self::wait_until_done).
    pub fn execute(&self, messages: impl Iterator<Item = M>, n_tasks: usize) {
        self.communicator
            .execution_progress()
            .add_to_pending_task_count(n_tasks);

        for message in messages {
            self.communicator
                .channel()
                .send_execute_instruction(message);
        }
    }

    /// Blocks the calling thread and returns as soon as all expected
    /// task executions have been performed.
    ///
    /// # Errors
    /// A [`ThreadPoolTaskErrors`] containing the [`TaskError`] of each
    /// failed task is returned if any of the executed tasks failed.
    pub fn wait_until_done(&self) -> ThreadPoolResult {
        self.communicator
            .execution_progress()
            .wait_for_no_pending_tasks();
        self.communicator.task_status().fetch_result()
    }
}

impl<M> Drop for ThreadPool<M> {
    fn drop(&mut self) {
        // Send a termination instruction for each of the workers
        for _ in 0..self.workers.len() {
            self.communicator
                .channel()
                .send_instruction(WorkerInstruction::Terminate);
        }

        // Join each worker as soon as it has terminated
        for worker in self.workers.drain(..) {
            worker.join();
        }
    }
}

impl TaskClosureReturnValue {
    /// Creates the return value corresponding to a successfully
    /// executed task.
    pub fn success() -> Self {
        Self::for_single_task(Ok(()))
    }

    /// Creates the return value corresponding to a failed
    /// execution of the given task with the given error.
    pub fn failure(task_id: TaskID, error: TaskError) -> Self {
        Self::for_single_task(Err((task_id, error)))
    }

    /// Increments the number of executed tasks by one in the
    /// given return value and returns the incremented version.
    pub fn with_incremented_task_count(self) -> Self {
        let Self {
            n_executed_tasks,
            result,
        } = self;
        Self {
            n_executed_tasks: n_executed_tasks + 1,
            result,
        }
    }

    /// Creates the return value corresponding to a single
    /// executed task with the given result.
    fn for_single_task(result: TaskClosureResult) -> Self {
        Self {
            n_executed_tasks: 1,
            result,
        }
    }
}

impl ThreadPoolTaskErrors {
    fn new(task_errors: HashMap<TaskID, TaskError>) -> Self {
        assert!(!task_errors.is_empty());
        Self {
            errors: task_errors,
        }
    }

    /// Returns the number of errors present from executed tasks
    /// that failed. Calling [`take_result_of`](Self::take_result_of)
    /// may reduce this number.
    pub fn n_errors(&self) -> usize {
        self.errors.len()
    }

    /// Returns a reference to the [`TaskError`] produced by the
    /// task with the given ID if the task executed and failed,
    /// otherwise returns [`None`].
    pub fn get_error_of(&self, task_id: TaskID) -> Option<&TaskError> {
        self.errors.get(&task_id)
    }

    /// Returns a [`Result`] that is either [`Ok`] if the task with
    /// the given ID succeeded or was never executed, or [`Err`]
    /// containing the resulting [`TaskError`] if it was executed
    /// and failed. In the latter case, the record if the error is
    /// removed from this object.
    pub fn take_result_of(&mut self, task_id: TaskID) -> Result<(), TaskError> {
        match self.errors.remove(&task_id) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl<M> ThreadPoolChannel<M> {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<WorkerInstruction<M>>();
        let receiver = Arc::new(Mutex::new(receiver));

        Self {
            owning_worker_id: None,
            sender,
            receiver,
        }
    }

    /// Returns the ID of the worker owning this instance of the
    /// [`ThreadPool`]'s channel.
    ///
    /// # Panics
    /// If called on a [`ThreadPoolChannel`] that has not been
    /// assigned to a worker thread.
    pub fn owning_worker_id(&self) -> WorkerID {
        self.owning_worker_id.unwrap()
    }

    /// Sends an instruction to execute the task with the given
    /// message to the recieving queue shared between the workers.
    /// Hence, the first available worker will execute the task once
    /// with the given message.
    pub fn send_execute_instruction(&self, message: M) {
        self.send_instruction(WorkerInstruction::Execute(message));
    }

    fn send_instruction(&self, message: WorkerInstruction<M>) {
        self.sender.send(message).unwrap();
    }

    fn wait_for_next_instruction(&self) -> WorkerInstruction<M> {
        self.receiver.lock().unwrap().recv().unwrap()
    }

    /// Creates a new instance of the channel for use by the
    /// given worker to exchange instructions with the other
    /// threads in the [`ThreadPool`].
    fn copy_for_worker(&self, worker_id: WorkerID) -> Self {
        Self {
            owning_worker_id: Some(worker_id),
            sender: self.sender.clone(),
            receiver: Arc::clone(&self.receiver),
        }
    }
}

impl<M> ThreadPoolCommunicator<M> {
    fn new(n_workers: NonZeroUsize) -> Self {
        let channel = ThreadPoolChannel::new();
        let execution_progress = ExecutionProgress::new();
        let task_status = TaskStatus::new();
        Self {
            n_workers,
            channel,
            execution_progress,
            task_status,
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

    fn task_status(&self) -> &TaskStatus {
        &self.task_status
    }

    /// Creates a new instance of the communicator for use by
    /// the given worker to communicate with the other threads
    /// in the [`ThreadPool`].
    fn copy_for_worker(&self, worker_id: WorkerID) -> Self {
        Self {
            n_workers: self.n_workers,
            channel: self.channel.copy_for_worker(worker_id),
            execution_progress: self.execution_progress.clone(),
            task_status: self.task_status.clone(),
        }
    }
}

impl ExecutionProgress {
    fn new() -> Self {
        let pending_task_count = Arc::new(AtomicUsize::new(0));
        let no_pending_tasks_condvar = Arc::new((Mutex::new(true), Condvar::new()));
        Self {
            pending_task_count,
            no_pending_tasks_condvar,
        }
    }

    /// Increments the atomic count of pending tasks by
    /// the given number and updates the conditional variable
    /// used for tracking whether there are pending tasks.
    fn add_to_pending_task_count(&self, n_tasks: usize) {
        log::debug!("Adding {} pending tasks", n_tasks);

        if n_tasks == 0 {
            return;
        }

        let previous_count = self.pending_task_count.fetch_add(n_tasks, Ordering::AcqRel);

        if previous_count == 0 {
            log::debug!("There are now pending tasks");
            self.set_no_pending_tasks(false);
            self.notify_change_of_no_pending_tasks();
        }
    }

    /// Decrements the atomic count of pending tasks by the
    /// given number and updates the conditional variable used
    /// for tracking whether there are pending tasks.
    fn register_executed_tasks(&self, worker_id: WorkerID, n_tasks: usize) {
        log::debug!(
            "Worker {} registering {} tasks as executed",
            worker_id,
            n_tasks
        );

        if n_tasks == 0 {
            return;
        }

        let previous_count = self.pending_task_count.fetch_sub(n_tasks, Ordering::AcqRel);
        assert!(
            previous_count >= n_tasks,
            "Underflow when registering executed tasks"
        );

        if previous_count == n_tasks {
            log::debug!("There are now no pending tasks");
            self.set_no_pending_tasks(true);
            self.notify_change_of_no_pending_tasks();
        }
    }

    /// Blocks execution in the calling thread and resumes when
    /// the count of pending tasks is zero.
    fn wait_for_no_pending_tasks(&self) {
        with_debug_logging!("Waiting for no pending tasks"; {
            let mut no_pending_tasks = self.no_pending_tasks_condvar.0.lock().unwrap();
            while !*no_pending_tasks {
                no_pending_tasks = self.no_pending_tasks_condvar.1.wait(no_pending_tasks).unwrap();
            }
        });
    }

    #[cfg(test)]
    fn pending_task_count(&self) -> usize {
        self.pending_task_count.load(Ordering::Acquire)
    }

    fn set_no_pending_tasks(&self, no_pending_tasks: bool) {
        *self.no_pending_tasks_condvar.0.lock().unwrap() = no_pending_tasks;
    }

    fn notify_change_of_no_pending_tasks(&self) {
        self.no_pending_tasks_condvar.1.notify_all();
    }
}

impl TaskStatus {
    fn new() -> Self {
        let some_task_failed = Arc::new(AtomicBool::new(false));
        let errors_of_failed_tasks = Arc::new(Mutex::new(HashMap::new()));
        Self {
            some_task_failed,
            errors_of_failed_tasks,
        }
    }

    fn fetch_result(&self) -> ThreadPoolResult {
        // Check if a task failed and at the same time reset the
        // flag to `false`
        if self.some_task_failed.swap(false, Ordering::Relaxed) {
            Err(ThreadPoolTaskErrors::new(
                // Move the `HashMap` of errors out of the mutex and
                // replace with an empty one
                std::mem::take(self.errors_of_failed_tasks.lock().unwrap().deref_mut()),
            ))
        } else {
            Ok(())
        }
    }

    fn register_error(&self, worker_id: WorkerID, task_id: TaskID, error: TaskError) {
        log::debug!(
            "Worker {} registered error on task {}: {}",
            worker_id,
            task_id,
            &error
        );
        self.some_task_failed.store(true, Ordering::Relaxed);
        self.errors_of_failed_tasks
            .lock()
            .unwrap()
            .insert(task_id, error);
    }
}

impl Worker {
    /// Spawns a new worker thread for executing the given
    /// task closure.
    fn spawn<M, F>(communicator: ThreadPoolCommunicator<M>, execute_tasks: &'static F) -> Self
    where
        M: Send + 'static,
        F: Fn(&ThreadPoolChannel<M>, M) -> TaskClosureReturnValue + Sync,
    {
        let handle = thread::spawn(move || {
            let worker_id = communicator.channel().owning_worker_id();
            log::debug!("Worker {} spawned", worker_id);

            loop {
                let instruction = communicator.channel().wait_for_next_instruction();

                match instruction {
                    WorkerInstruction::Execute(message) => {
                        let TaskClosureReturnValue {
                            n_executed_tasks,
                            result,
                        } = execute_tasks(communicator.channel(), message);

                        if let Err((task_id, error)) = result {
                            communicator
                                .task_status()
                                .register_error(worker_id, task_id, error);
                        }

                        communicator
                            .execution_progress()
                            .register_executed_tasks(worker_id, n_executed_tasks);
                    }
                    WorkerInstruction::Terminate => {
                        log::debug!("Worker {} terminating", worker_id);
                        return;
                    }
                }
            }
        });
        Self { handle }
    }

    fn join(self) {
        self.handle.join().unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::anyhow;
    use std::iter;

    struct NoMessage;

    #[test]
    fn creating_thread_communicator_works() {
        let n_workers = 2;
        let comm = ThreadPoolCommunicator::<NoMessage>::new(NonZeroUsize::new(n_workers).unwrap());
        assert_eq!(comm.n_workers().get(), n_workers);
    }

    #[test]
    fn sending_message_with_communicator_works() {
        let n_workers = 1;
        let comm = ThreadPoolCommunicator::new(NonZeroUsize::new(n_workers).unwrap());
        comm.channel().send_execute_instruction(42);
        let message = comm.channel().wait_for_next_instruction();
        assert_eq!(message, WorkerInstruction::Execute(42));
    }

    #[test]
    fn keeping_track_of_pending_task_count_works() {
        let n_workers = 1;
        let comm = ThreadPoolCommunicator::<NoMessage>::new(NonZeroUsize::new(n_workers).unwrap());
        assert_eq!(comm.execution_progress().pending_task_count(), 0);
        comm.execution_progress().add_to_pending_task_count(2);
        assert_eq!(comm.execution_progress().pending_task_count(), 2);
        comm.execution_progress().add_to_pending_task_count(1);
        assert_eq!(comm.execution_progress().pending_task_count(), 3);

        comm.execution_progress().register_executed_tasks(0, 2);
        assert_eq!(comm.execution_progress().pending_task_count(), 1);
        comm.execution_progress().register_executed_tasks(0, 1);
        assert_eq!(comm.execution_progress().pending_task_count(), 0);

        comm.execution_progress().wait_for_no_pending_tasks(); // Should return immediately
    }

    #[test]
    #[should_panic]
    fn registering_executed_task_when_none_are_pending_fails() {
        let n_workers = 2;
        let comm = ThreadPoolCommunicator::<NoMessage>::new(NonZeroUsize::new(n_workers).unwrap());
        comm.execution_progress().register_executed_tasks(0, 1);
    }

    #[test]
    fn creating_thread_pool_works() {
        let n_workers = 2;
        let pool = ThreadPool::<NoMessage>::new(NonZeroUsize::new(n_workers).unwrap(), &|_, _| {
            TaskClosureReturnValue::success()
        });
        assert_eq!(pool.n_workers().get(), n_workers);
    }

    #[test]
    fn executing_thread_pool_works() {
        let n_workers = 2;
        let count = Arc::new(Mutex::new(0));
        let pool = ThreadPool::new(NonZeroUsize::new(n_workers).unwrap(), &|_,
                                                                            (count, incr): (
            Arc<Mutex<usize>>,
            usize,
        )| {
            *count.lock().unwrap() += incr;
            TaskClosureReturnValue::success()
        });
        pool.execute_and_wait(
            iter::repeat_with(|| (Arc::clone(&count), 3)).take(n_workers),
            n_workers,
        )
        .unwrap();
        drop(pool);
        assert_eq!(*count.lock().unwrap(), n_workers * 3);
    }

    #[test]
    fn capturing_task_error_works() {
        let n_workers = 2;
        let count = Arc::new(Mutex::new(1));
        let pool = ThreadPool::new(NonZeroUsize::new(n_workers).unwrap(), &|_,
                                                                            (
            count,
            task_id,
        ): (
            Arc<Mutex<usize>>,
            TaskID,
        )| {
            let mut count = count.lock().unwrap();

            // The second of the two tasks will cause underflow
            match count.checked_sub(1) {
                Some(decremented_count) => {
                    *count = decremented_count;
                    TaskClosureReturnValue::success()
                }
                None => TaskClosureReturnValue::failure(task_id, anyhow!("Underflow!")),
            }
        });
        let result = pool.execute_and_wait(
            [(Arc::clone(&count), 0), (Arc::clone(&count), 1)].into_iter(),
            2,
        );
        assert!(result.is_err());

        let mut errors = result.err().unwrap();

        assert_eq!(errors.n_errors(), 1);

        match (errors.take_result_of(0), errors.take_result_of(1)) {
            (Err(err), Ok(_)) | (Ok(_), Err(err)) => assert_eq!(err.to_string(), "Underflow!"),
            _ => unreachable!(),
        }

        assert_eq!(errors.n_errors(), 0);
    }
}
