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
pub struct ThreadPool {
    communicator: ThreadCommunicator,
    workers: Vec<Worker>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Message {
    Execute,
    Terminate,
}

#[derive(Clone, Debug)]
pub struct ThreadCommunicator {
    n_workers: usize,
    n_idle_workers: Arc<AtomicUsize>,
    all_workers_idle_condvar: Arc<(Mutex<bool>, Condvar)>,
    sender: Sender<Message>,
    receiver: Arc<Mutex<Receiver<Message>>>,
}

#[derive(Debug)]
struct Worker {
    handle: JoinHandle<()>,
}

impl ThreadPool {
    pub fn new<A, S>(n_workers: NonZeroUsize, action: A, state: Arc<S>) -> Self
    where
        A: Fn(&ThreadCommunicator, &S) + Copy + Send + 'static,
        S: Sync + Send + 'static,
    {
        let communicator = ThreadCommunicator::new(n_workers);

        let workers = (0..n_workers.get())
            .into_iter()
            .map(|_| Worker::spawn(communicator.clone(), action, Arc::clone(&state)))
            .collect();

        Self {
            communicator,
            workers,
        }
    }

    pub fn n_workers(&self) -> usize {
        self.communicator.n_workers()
    }

    pub fn execute(&self) {
        self.execute_with_workers();
        self.wait_for_all_workers_idle();
    }

    pub fn execute_with_workers(&self) {
        self.send_message_to_all_workers(Message::Execute);
        self.communicator.set_all_workers_idle(false);
    }

    pub fn wait_for_all_workers_idle(&self) {
        self.communicator.wait_for_all_workers_idle();
    }

    fn send_message_to_all_workers(&self, message: Message) {
        for _ in 0..self.workers.len() {
            self.communicator.send_message(message);
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.send_message_to_all_workers(Message::Terminate);

        for worker in self.workers.drain(..) {
            worker.join();
        }
    }
}

impl ThreadCommunicator {
    fn new(n_workers: NonZeroUsize) -> Self {
        let n_workers = n_workers.get();

        let (sender, receiver) = mpsc::channel::<Message>();
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

    pub fn send_execute_message(&self) {
        self.send_message(Message::Execute);
    }

    fn send_message(&self, message: Message) {
        self.sender.send(message).unwrap();
    }

    fn receive_message(&self) -> Message {
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
    fn spawn<A, S>(communicator: ThreadCommunicator, action: A, state: Arc<S>) -> Self
    where
        A: Fn(&ThreadCommunicator, &S) + Send + 'static,
        S: Sync + Send + 'static,
    {
        let handle = thread::spawn(move || loop {
            let message = communicator.receive_message();

            match message {
                Message::Execute => {
                    communicator.register_busy_worker();
                    action(&communicator, state.as_ref());
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

    #[test]
    fn creating_thread_communicator_works() {
        let n_workers = 2;
        let communicator = ThreadCommunicator::new(NonZeroUsize::new(n_workers).unwrap());
        assert_eq!(communicator.n_workers(), n_workers);
    }

    #[test]
    fn sending_message_with_communicator_works() {
        let n_workers = 1;
        let communicator = ThreadCommunicator::new(NonZeroUsize::new(n_workers).unwrap());
        communicator.send_message(Message::Execute);
        let message = communicator.receive_message();
        assert_eq!(message, Message::Execute);
    }

    #[test]
    fn keeping_track_of_idle_workers_works() {
        let communicator = ThreadCommunicator::new(NonZeroUsize::new(2).unwrap());
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
        let communicator = ThreadCommunicator::new(NonZeroUsize::new(n_workers).unwrap());
        communicator.register_idle_worker();
    }

    #[test]
    #[should_panic]
    fn registering_busy_worker_when_all_are_busy_fails() {
        let communicator = ThreadCommunicator::new(NonZeroUsize::new(1).unwrap());
        communicator.register_busy_worker();
        communicator.register_busy_worker(); // Should panic here
    }

    #[test]
    fn creating_thread_pool_works() {
        let n_workers = 2;
        let pool = ThreadPool::new(
            NonZeroUsize::new(n_workers).unwrap(),
            |_, _| {},
            Arc::new(()),
        );
        assert_eq!(pool.n_workers(), n_workers);
    }

    #[test]
    fn executing_thread_pool_works() {
        let n_workers = 2;
        let count = Arc::new(Mutex::new(0));
        let pool = ThreadPool::new(
            NonZeroUsize::new(n_workers).unwrap(),
            |_, count| *count.lock().unwrap() += 1,
            Arc::clone(&count),
        );
        pool.execute();
        drop(pool);
        assert_eq!(*count.lock().unwrap(), n_workers);
    }
}
