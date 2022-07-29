//! Utilities for multithreading.

use std::{
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
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
///     &|_comm, (count, incr): (Arc<Mutex<usize>>, usize)| {
///         *count.lock().unwrap() += incr
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
/// pool.execute_and_wait(messages);
///
/// assert_eq!(*count.lock().unwrap(), n_workers * incr);
/// ```
///
/// # Type parameters
/// `M` is the type of message content sent to threads
/// when they should execute a task.
#[derive(Debug)]
pub struct ThreadPool<M> {
    communicator: ThreadCommunicator<M>,
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

/// A shared structure  for handling communication between
/// the threads in a [`ThreadPool`].
#[derive(Debug)]
pub struct ThreadCommunicator<M> {
    worker_id: Option<WorkerID>,
    n_workers: usize,
    n_idle_workers: Arc<AtomicUsize>,
    all_workers_idle_condvar: Arc<(Mutex<bool>, Condvar)>,
    sender: Sender<WorkerInstruction<M>>,
    receiver: Arc<Mutex<Receiver<WorkerInstruction<M>>>>,
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
    /// [`ThreadCommunicator`] that can be used to send messages
    /// to other threads from the closure.
    pub fn new<T>(n_workers: NonZeroUsize, execute_task: &'static T) -> Self
    where
        M: Send + 'static,
        T: Fn(&ThreadCommunicator<M>, M) + Sync,
    {
        let communicator = ThreadCommunicator::new(n_workers);

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
    /// have been completed. To avoid blocking the main thread, use
    /// [`execute`](Self::execute) instead.
    pub fn execute_and_wait(&self, messages: impl Iterator<Item = M>) {
        self.execute(messages);
        self.wait_until_done();
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
                self.communicator.set_all_workers_idle(false);
            }
            self.communicator.send_execute_instruction(message);
        }
    }

    /// Blocks the calling thread and returns as soon as all pending
    /// and currently executing task in the pool have been completed.
    pub fn wait_until_done(&self) {
        self.communicator.wait_for_all_workers_idle();
    }
}

impl<M> Drop for ThreadPool<M> {
    fn drop(&mut self) {
        // Send a termination instruction for each of the workers
        for _ in 0..self.workers.len() {
            self.communicator
                .send_instruction(WorkerInstruction::Terminate);
        }

        // Join each worker as soon as it has terminated
        for worker in self.workers.drain(..) {
            worker.join();
        }
    }
}

impl<M> ThreadCommunicator<M> {
    fn new(n_workers: NonZeroUsize) -> Self {
        let n_workers = n_workers.get();

        let (sender, receiver) = mpsc::channel::<WorkerInstruction<M>>();
        let receiver = Arc::new(Mutex::new(receiver));

        let n_idle_workers = Arc::new(AtomicUsize::new(n_workers));
        let all_workers_idle_condvar = Arc::new((Mutex::new(true), Condvar::new()));

        Self {
            worker_id: None,
            n_workers,
            n_idle_workers,
            all_workers_idle_condvar,
            sender,
            receiver,
        }
    }

    /// Returns the ID of the worker owning this instance of
    /// the [`ThreadPool`]'s communicator.
    ///
    /// # Panics
    /// If called on a [`ThreadCommunicator`] that has not been
    /// assigned to a worker thread.
    pub fn worker_id(&self) -> WorkerID {
        self.worker_id.unwrap()
    }

    /// Returns the number of worker threads in the thread pool
    /// (this does not include the main thread).
    pub fn n_workers(&self) -> usize {
        self.n_workers
    }

    /// Sends an instruction to execute the task with the given
    /// message to the recieving queue shared between the workers.
    /// Hence, the first available worker will execute the task once
    /// with the given message.
    pub fn send_execute_instruction(&self, message: M) {
        self.send_instruction(WorkerInstruction::Execute(message));
    }

    /// Creates a new instance of the communicator that can be
    /// used by the given worker to communicate with the other
    /// threads in the [`ThreadPool`].
    fn copy_for_worker(&self, worker_id: WorkerID) -> Self {
        Self {
            worker_id: Some(worker_id),
            n_workers: self.n_workers,
            n_idle_workers: self.n_idle_workers.clone(),
            all_workers_idle_condvar: self.all_workers_idle_condvar.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }

    fn send_instruction(&self, message: WorkerInstruction<M>) {
        self.sender.send(message).unwrap();
    }

    fn receive_message(&self) -> WorkerInstruction<M> {
        self.receiver.lock().unwrap().recv().unwrap()
    }

    /// Increments the atomic count of idle workers and
    /// updates the conditional variable used for tracking
    /// whether all workers are idle.
    fn register_idle_worker(&self) {
        let previous_count = self.n_idle_workers.fetch_add(1, Ordering::AcqRel);
        assert!(previous_count < self.n_workers());

        // If all workers are now idle, we must update the associated
        // conditional variable
        if previous_count + 1 == self.n_workers() {
            self.set_all_workers_idle(true);
            // Threads waiting for complete idleness to change should be notified
            self.all_workers_idle_condvar.1.notify_all();
        }
    }

    /// Decrements the atomic count of idle workers and
    /// updates the conditional variable used for tracking
    /// whether all workers are idle.
    fn register_busy_worker(&self) {
        let previous_count = self.n_idle_workers.fetch_sub(1, Ordering::AcqRel);
        assert_ne!(previous_count, 0);

        // If all workers are no longer idle, we must update the associated
        // conditional variable
        if previous_count == self.n_workers() {
            self.set_all_workers_idle(false);
            // Threads waiting for complete idleness to change should be notified
            self.all_workers_idle_condvar.1.notify_all();
        }
    }

    fn set_all_workers_idle(&self, all_idle: bool) {
        *self.all_workers_idle_condvar.0.lock().unwrap() = all_idle;
    }

    #[allow(dead_code)]
    fn n_idle_workers(&self) -> usize {
        self.n_idle_workers.load(Ordering::Acquire)
    }

    /// Blocks execution in the calling thread and returns when
    /// all worker threads are idle.
    fn wait_for_all_workers_idle(&self) {
        let mut all_idle = self.all_workers_idle_condvar.0.lock().unwrap();
        while !*all_idle {
            all_idle = self.all_workers_idle_condvar.1.wait(all_idle).unwrap();
        }
    }
}

impl Worker {
    /// Spawns a new worker thread for executing the given
    /// task.
    fn spawn<M, T>(communicator: ThreadCommunicator<M>, execute_task: &'static T) -> Self
    where
        M: Send + 'static,
        T: Fn(&ThreadCommunicator<M>, M) + Sync,
    {
        let handle = thread::spawn(move || loop {
            let message = communicator.receive_message();

            match message {
                WorkerInstruction::Execute(message) => {
                    communicator.register_busy_worker();
                    execute_task(&communicator, message);
                    communicator.register_idle_worker();
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
    use std::iter;

    #[test]
    fn creating_thread_communicator_works() {
        let n_workers = 2;
        let communicator = ThreadCommunicator::<()>::new(NonZeroUsize::new(n_workers).unwrap());
        assert_eq!(communicator.n_workers(), n_workers);
    }

    #[test]
    fn sending_message_with_communicator_works() {
        let n_workers = 1;
        let communicator = ThreadCommunicator::new(NonZeroUsize::new(n_workers).unwrap());
        communicator.send_execute_instruction(42);
        let message = communicator.receive_message();
        assert_eq!(message, WorkerInstruction::Execute(42));
    }

    #[test]
    fn keeping_track_of_idle_workers_works() {
        let communicator = ThreadCommunicator::<()>::new(NonZeroUsize::new(2).unwrap());
        assert_eq!(communicator.n_idle_workers(), 2);
        communicator.register_busy_worker();
        assert_eq!(communicator.n_idle_workers(), 1);
        communicator.register_busy_worker();
        assert_eq!(communicator.n_idle_workers(), 0);

        communicator.register_idle_worker();
        assert_eq!(communicator.n_idle_workers(), 1);
        communicator.register_idle_worker();
        assert_eq!(communicator.n_idle_workers(), 2);

        communicator.wait_for_all_workers_idle(); // Should return immediately
    }

    #[test]
    #[should_panic]
    fn registering_idle_worker_when_all_are_idle_fails() {
        let n_workers = 2;
        let communicator = ThreadCommunicator::<()>::new(NonZeroUsize::new(n_workers).unwrap());
        communicator.register_idle_worker();
    }

    #[test]
    #[should_panic]
    fn registering_busy_worker_when_all_are_busy_fails() {
        let communicator = ThreadCommunicator::<()>::new(NonZeroUsize::new(1).unwrap());
        communicator.register_busy_worker();
        communicator.register_busy_worker(); // Should panic here
    }

    #[test]
    fn creating_thread_pool_works() {
        let n_workers = 2;
        let pool = ThreadPool::<()>::new(NonZeroUsize::new(n_workers).unwrap(), &|_, _| {});
        assert_eq!(pool.n_workers(), n_workers);
    }

    #[test]
    fn executing_thread_pool_works() {
        let n_workers = 2;
        let count = Arc::new(Mutex::new(0));
        let pool = ThreadPool::new(NonZeroUsize::new(n_workers).unwrap(), &|_,
                                                                            (count, incr): (
            Arc<Mutex<_>>,
            _,
        )| {
            *count.lock().unwrap() += incr
        });
        pool.execute_and_wait(iter::repeat_with(|| (Arc::clone(&count), 3)).take(n_workers));
        drop(pool);
        assert_eq!(*count.lock().unwrap(), n_workers * 3);
    }
}
