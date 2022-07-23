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

#[derive(Debug)]
pub struct ThreadPool<M> {
    communicator: ThreadCommunicator<M>,
    workers: Vec<Worker>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Message<M> {
    Execute(M),
    Terminate,
}

#[derive(Clone, Debug)]
pub struct ThreadCommunicator<M> {
    n_workers: usize,
    n_idle_workers: Arc<AtomicUsize>,
    all_workers_idle_condvar: Arc<(Mutex<bool>, Condvar)>,
    sender: Sender<Message<M>>,
    receiver: Arc<Mutex<Receiver<Message<M>>>>,
}

#[derive(Debug)]
struct Worker {
    handle: JoinHandle<()>,
}

impl<M> ThreadPool<M> {
    pub fn new<S, A>(n_workers: NonZeroUsize, state: Arc<S>, action: A) -> Self
    where
        M: Clone + Send + 'static,
        S: Send + Sync + 'static,
        A: Fn(&ThreadCommunicator<M>, &S, M) + Copy + Send + 'static,
    {
        let communicator = ThreadCommunicator::new(n_workers);

        let workers = (0..n_workers.get())
            .into_iter()
            .map(|_| Worker::spawn(communicator.clone(), Arc::clone(&state), action))
            .collect();

        Self {
            communicator,
            workers,
        }
    }

    pub fn new_stateless<A>(n_workers: NonZeroUsize, action: A) -> Self
    where
        M: Clone + Send + 'static,
        A: Fn(&ThreadCommunicator<M>, M) + Copy + Send + 'static,
    {
        Self::new(n_workers, Arc::new(()), move |comm, _, message| {
            action(comm, message)
        })
    }

    pub fn n_workers(&self) -> usize {
        self.communicator.n_workers()
    }

    pub fn execute(&self, messages: impl Iterator<Item = M>) {
        self.execute_with_workers(messages);
        self.wait_for_all_workers_idle();
    }

    pub fn execute_with_workers(&self, messages: impl Iterator<Item = M>) {
        for message in messages {
            self.communicator.send_execute_message(message);
        }
        self.communicator.set_all_workers_idle(false);
    }

    pub fn wait_for_all_workers_idle(&self) {
        self.communicator.wait_for_all_workers_idle();
    }
}

impl<M> Drop for ThreadPool<M> {
    fn drop(&mut self) {
        for _ in 0..self.workers.len() {
            self.communicator.send_message(Message::Terminate);
        }

        for worker in self.workers.drain(..) {
            worker.join();
        }
    }
}

impl<M> ThreadCommunicator<M> {
    fn new(n_workers: NonZeroUsize) -> Self {
        let n_workers = n_workers.get();

        let (sender, receiver) = mpsc::channel::<Message<M>>();
        let receiver = Arc::new(Mutex::new(receiver));

        let n_idle_workers = Arc::new(AtomicUsize::new(n_workers));
        let all_workers_idle_condvar = Arc::new((Mutex::new(true), Condvar::new()));

        Self {
            n_workers,
            n_idle_workers,
            all_workers_idle_condvar,
            sender,
            receiver,
        }
    }

    pub fn n_workers(&self) -> usize {
        self.n_workers
    }

    pub fn send_execute_message(&self, message: M) {
        self.send_message(Message::Execute(message));
    }

    fn send_message(&self, message: Message<M>) {
        self.sender.send(message).unwrap();
    }

    fn receive_message(&self) -> Message<M> {
        self.receiver.lock().unwrap().recv().unwrap()
    }

    fn register_idle_worker(&self) {
        let previous_count = self.n_idle_workers.fetch_add(1, Ordering::AcqRel);
        assert!(previous_count < self.n_workers());
        if previous_count + 1 == self.n_workers() {
            self.set_all_workers_idle(true);
            self.all_workers_idle_condvar.1.notify_all();
        }
    }

    fn register_busy_worker(&self) {
        let previous_count = self.n_idle_workers.fetch_sub(1, Ordering::AcqRel);
        assert_ne!(previous_count, 0);
        if previous_count == self.n_workers() {
            self.set_all_workers_idle(false);
            self.all_workers_idle_condvar.1.notify_all();
        }
    }

    fn set_all_workers_idle(&self, all_idle: bool) {
        *self.all_workers_idle_condvar.0.lock().unwrap() = all_idle;
    }

    fn n_idle_workers(&self) -> usize {
        self.n_idle_workers.load(Ordering::Acquire)
    }

    fn wait_for_all_workers_idle(&self) {
        let mut all_idle = self.all_workers_idle_condvar.0.lock().unwrap();
        while !*all_idle {
            all_idle = self.all_workers_idle_condvar.1.wait(all_idle).unwrap();
        }
    }
}

impl Worker {
    fn spawn<M, S, A>(communicator: ThreadCommunicator<M>, state: Arc<S>, action: A) -> Self
    where
        M: Send + 'static,
        S: Send + Sync + 'static,
        A: Fn(&ThreadCommunicator<M>, &S, M) + Send + 'static,
    {
        let handle = thread::spawn(move || loop {
            let message = communicator.receive_message();

            match message {
                Message::Execute(message) => {
                    communicator.register_busy_worker();
                    action(&communicator, state.as_ref(), message);
                    communicator.register_idle_worker();
                }
                Message::Terminate => {
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
        communicator.send_execute_message(42);
        let message = communicator.receive_message();
        assert_eq!(message, Message::Execute(42));
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
        let pool =
            ThreadPool::<()>::new_stateless(NonZeroUsize::new(n_workers).unwrap(), |_, _| {});
        assert_eq!(pool.n_workers(), n_workers);
    }

    #[test]
    fn executing_thread_pool_works() {
        let n_workers = 2;
        let count = Arc::new(Mutex::new(0));
        let pool = ThreadPool::new(
            NonZeroUsize::new(n_workers).unwrap(),
            Arc::clone(&count),
            |_, count, incr| *count.lock().unwrap() += incr,
        );
        pool.execute(iter::repeat(3).take(n_workers));
        drop(pool);
        assert_eq!(*count.lock().unwrap(), n_workers * 3);
    }
}
