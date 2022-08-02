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
/// # use impact::thread::ThreadPool;
/// # use std::{iter, num::NonZeroUsize, sync::{Arc, Mutex}};
/// #
/// let n_workers = 2;
/// let pool = ThreadPool::new(
///     // At least one worker is required
///     NonZeroUsize::new(n_workers).unwrap(),
///     // Define task that increments a shared count
///     &|_channel, (count, incr): (Arc<Mutex<usize>>, usize)| {
///         *count.lock().unwrap() += incr;
///         Ok(()) // The closure must return a `Result<(), (TaskID, TaskError)>`
///     }
/// );
///
/// // Create shared mutex with initial count
/// let count = Arc::new(Mutex::new(0));
/// // Amount to increment count
/// let incr = 3;
///
/// // Create one message for each worker, each containing
/// // a reference to the shared count and the increment
/// let messages = iter::repeat_with(|| (Arc::clone(&count), incr)).take(n_workers);
///
/// // Execute the task once with each message and wait until done
/// pool.execute_and_wait(messages).unwrap();
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

/// The type if ID used for workers in a [`ThreadPool`].
pub type WorkerID = usize;

/// Type of ID used for identifying tasks that can be performed
/// by worker threads in a [`ThreadPool`].
pub type TaskID = u64;

/// [`Result`] returned by executed tasks. The [`Err`]
/// variant contains the [`TaskID`] together with the
/// [`TaskError`].
pub type TaskResult = Result<(), (TaskID, TaskError)>;

/// Type of error produced by failed task executions.
pub type TaskError = Error;

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
    channel: ThreadPoolChannel<M>,
    worker_status: WorkerStatus,
    task_status: TaskStatus,
}

#[derive(Clone, Debug)]
struct WorkerStatus {
    n_workers: usize,
    n_idle_workers: Arc<AtomicUsize>,
    all_workers_idle_condvar: Arc<(Mutex<bool>, Condvar)>,
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
        T: Fn(&ThreadPoolChannel<M>, M) -> TaskResult + Sync,
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
    pub fn n_workers(&self) -> usize {
        self.communicator.n_workers()
    }

    /// Instructs worker threads in the pool to execute their task.
    /// The task will be executed with each of the given messages.
    /// This function does not return until all the task executions
    /// have been completed or have failed with an error. To avoid
    /// blocking the main thread, use [`execute`](Self::execute)
    /// instead.
    ///
    /// # Errors
    /// A [`ThreadPoolTaskErrors`] containing the [`TaskError`] of each
    /// failed task is returned if any of the executed tasks failed.
    pub fn execute_and_wait(&self, messages: impl Iterator<Item = M>) -> ThreadPoolResult {
        self.execute(messages);
        self.wait_until_done()
    }

    /// Instructs worker threads in the pool to execute their task.
    /// The task will be executed with each of the given messages.
    /// This function returns as soon as all the execution instructions
    /// have been sendt. To block until all tasks have been completed,
    /// call [`wait_until_done`](Self::wait_until_done).
    pub fn execute(&self, messages: impl Iterator<Item = M>) {
        for (idx, message) in messages.enumerate() {
            // If at least one execution message is sent, ensure
            // that the `all_workers_idle` flag is set to `false`
            // immediately so that a potential call to
            // `wait_for_all_workers_idle` before any workers have
            // actually had time to register as busy does not return
            // immediately.
            if idx == 0 {
                self.communicator.worker_status().set_all_idle(false);
            }
            self.communicator
                .channel()
                .send_execute_instruction(message);
        }
    }

    /// Blocks the calling thread and returns as soon as all pending
    /// and currently executing task in the pool have been completed
    /// or have failed with an error.
    ///
    /// # Errors
    /// A [`ThreadPoolTaskErrors`] containing the [`TaskError`] of each
    /// failed task is returned if any of the executed tasks failed.
    pub fn wait_until_done(&self) -> ThreadPoolResult {
        self.communicator.worker_status().wait_for_all_idle();
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

impl ThreadPoolTaskErrors {
    fn new(task_errors: HashMap<TaskID, TaskError>) -> Self {
        assert!(!task_errors.is_empty());
        Self {
            errors: task_errors,
        }
    }

    /// Returns the number of executed tasks that failed with
    /// and error.
    pub fn n_failed_tasks(&self) -> usize {
        self.errors.len()
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

    fn receive_message(&self) -> WorkerInstruction<M> {
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
        let worker_status = WorkerStatus::new_all_idle(n_workers.get());
        let task_status = TaskStatus::new();
        Self {
            channel,
            worker_status,
            task_status,
        }
    }

    fn n_workers(&self) -> usize {
        self.worker_status.n_workers()
    }

    fn channel(&self) -> &ThreadPoolChannel<M> {
        &self.channel
    }

    fn worker_status(&self) -> &WorkerStatus {
        &self.worker_status
    }

    fn task_status(&self) -> &TaskStatus {
        &self.task_status
    }

    /// Creates a new instance of the communicator for use by
    /// the given worker to communicate with the other threads
    /// in the [`ThreadPool`].
    fn copy_for_worker(&self, worker_id: WorkerID) -> Self {
        Self {
            channel: self.channel.copy_for_worker(worker_id),
            worker_status: self.worker_status.clone(),
            task_status: self.task_status.clone(),
        }
    }
}

impl WorkerStatus {
    fn new_all_idle(n_workers: usize) -> Self {
        Self::new(n_workers, n_workers)
    }

    fn new(n_workers: usize, n_idle_workers: usize) -> Self {
        let all_idle = n_idle_workers == n_workers;
        let n_idle_workers = Arc::new(AtomicUsize::new(n_idle_workers));
        let all_workers_idle_condvar = Arc::new((Mutex::new(all_idle), Condvar::new()));
        Self {
            n_workers,
            n_idle_workers,
            all_workers_idle_condvar,
        }
    }

    fn n_workers(&self) -> usize {
        self.n_workers
    }

    #[cfg(test)]
    fn n_idle(&self) -> usize {
        self.n_idle_workers.load(Ordering::Acquire)
    }

    fn set_all_idle(&self, all_idle: bool) {
        *self.all_workers_idle_condvar.0.lock().unwrap() = all_idle;
    }

    /// Increments the atomic count of idle workers and
    /// updates the conditional variable used for tracking
    /// whether all workers are idle.
    fn register_idle(&self) {
        let previous_count = self.n_idle_workers.fetch_add(1, Ordering::AcqRel);
        assert!(previous_count < self.n_workers());

        // If all workers are now idle, we must update the associated
        // conditional variable
        if previous_count + 1 == self.n_workers() {
            self.set_all_idle(true);
            // Threads waiting for complete idleness to change should be notified
            self.all_workers_idle_condvar.1.notify_all();
        }
    }

    /// Decrements the atomic count of idle workers and
    /// updates the conditional variable used for tracking
    /// whether all workers are idle.
    fn register_busy(&self) {
        let previous_count = self.n_idle_workers.fetch_sub(1, Ordering::AcqRel);
        assert_ne!(previous_count, 0);

        // If all workers are no longer idle, we must update the associated
        // conditional variable
        if previous_count == self.n_workers() {
            self.set_all_idle(false);
            // Threads waiting for complete idleness to change should be notified
            self.all_workers_idle_condvar.1.notify_all();
        }
    }

    /// Blocks execution in the calling thread and returns when
    /// all worker threads are idle.
    fn wait_for_all_idle(&self) {
        let mut all_idle = self.all_workers_idle_condvar.0.lock().unwrap();
        while !*all_idle {
            all_idle = self.all_workers_idle_condvar.1.wait(all_idle).unwrap();
        }
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

    fn register_error(&self, task_id: TaskID, error: TaskError) {
        self.some_task_failed.store(true, Ordering::Relaxed);
        self.errors_of_failed_tasks
            .lock()
            .unwrap()
            .insert(task_id, error);
    }
}

impl Worker {
    /// Spawns a new worker thread for executing the given
    /// task.
    fn spawn<M, T>(communicator: ThreadPoolCommunicator<M>, execute_task: &'static T) -> Self
    where
        M: Send + 'static,
        T: Fn(&ThreadPoolChannel<M>, M) -> TaskResult + Sync,
    {
        let handle = thread::spawn(move || loop {
            let message = communicator.channel().receive_message();

            match message {
                WorkerInstruction::Execute(message) => {
                    communicator.worker_status().register_busy();
                    if let Err((task_id, error)) = execute_task(communicator.channel(), message) {
                        communicator.task_status().register_error(task_id, error);
                    }
                    communicator.worker_status().register_idle();
                }
                WorkerInstruction::Terminate => {
                    return;
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
        assert_eq!(comm.n_workers(), n_workers);
    }

    #[test]
    fn sending_message_with_communicator_works() {
        let n_workers = 1;
        let comm = ThreadPoolCommunicator::new(NonZeroUsize::new(n_workers).unwrap());
        comm.channel().send_execute_instruction(42);
        let message = comm.channel().receive_message();
        assert_eq!(message, WorkerInstruction::Execute(42));
    }

    #[test]
    fn keeping_track_of_idle_workers_works() {
        let comm = ThreadPoolCommunicator::<NoMessage>::new(NonZeroUsize::new(2).unwrap());
        assert_eq!(comm.worker_status().n_idle(), 2);
        comm.worker_status().register_busy();
        assert_eq!(comm.worker_status().n_idle(), 1);
        comm.worker_status().register_busy();
        assert_eq!(comm.worker_status().n_idle(), 0);

        comm.worker_status().register_idle();
        assert_eq!(comm.worker_status().n_idle(), 1);
        comm.worker_status().register_idle();
        assert_eq!(comm.worker_status().n_idle(), 2);

        comm.worker_status().wait_for_all_idle(); // Should return immediately
    }

    #[test]
    #[should_panic]
    fn registering_idle_worker_when_all_are_idle_fails() {
        let n_workers = 2;
        let comm = ThreadPoolCommunicator::<NoMessage>::new(NonZeroUsize::new(n_workers).unwrap());
        comm.worker_status().register_idle();
    }

    #[test]
    #[should_panic]
    fn registering_busy_worker_when_all_are_busy_fails() {
        let comm = ThreadPoolCommunicator::<NoMessage>::new(NonZeroUsize::new(1).unwrap());
        comm.worker_status().register_busy();
        comm.worker_status().register_busy(); // Should panic here
    }

    #[test]
    fn creating_thread_pool_works() {
        let n_workers = 2;
        let pool =
            ThreadPool::<NoMessage>::new(NonZeroUsize::new(n_workers).unwrap(), &|_, _| Ok(()));
        assert_eq!(pool.n_workers(), n_workers);
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
            Ok(())
        });
        pool.execute_and_wait(iter::repeat_with(|| (Arc::clone(&count), 3)).take(n_workers))
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
            // The second of the two tasks will cause underflow here
            let decremented_count = count
                .checked_sub(1)
                .ok_or_else(|| (task_id, anyhow!("Underflow!")))?;
            *count = decremented_count;
            Ok(())
        });
        let result =
            pool.execute_and_wait([(Arc::clone(&count), 0), (Arc::clone(&count), 1)].into_iter());
        assert!(result.is_err());

        let mut errors = result.err().unwrap();

        assert_eq!(errors.n_failed_tasks(), 1);

        match (errors.take_result_of(0), errors.take_result_of(1)) {
            (Err(err), Ok(_)) | (Ok(_), Err(err)) => assert_eq!(err.to_string(), "Underflow!"),
            _ => unreachable!(),
        }

        assert_eq!(errors.n_failed_tasks(), 0);
    }
}
