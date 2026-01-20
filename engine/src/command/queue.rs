//! Command queueing.

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::VecDeque;

/// A buffer for queueing incoming commands until they are ready for execution.
#[derive(Debug)]
pub struct CommandQueue<C> {
    commands: RwLock<VecDeque<C>>,
}

impl<C> CommandQueue<C> {
    /// Creates an empty command queue.
    pub const fn new() -> Self {
        Self {
            commands: RwLock::new(VecDeque::new()),
        }
    }

    /// Adds the given command to the back of the queue.
    pub fn enqueue_command(&self, command: C) {
        self.commands.write().push_back(command);
    }

    /// Uses the given closure to execute each command in the queue, in the
    /// order they were inserted. Upon execution, the command is removed from
    /// the queue.
    ///
    /// # Concurrency
    /// If the closure causes [`Self::enqueue_command`] or
    /// [`Self::execute_commands`] to be called, it will deadlock.
    pub fn execute_commands(&self, mut execute: impl FnMut(C)) {
        let mut commands = self.commands.write();
        while let Some(command) = commands.pop_front() {
            execute(command);
        }
    }

    /// Uses the given closure to execute each command in the queue, in the
    /// order they were inserted. Upon execution, the command is removed from
    /// the queue. If the closure returns an error, it will be returned
    /// immediately and the remaining commands will remain in the queue.
    ///
    /// # Concurrency
    /// If the closure causes [`Self::enqueue_command`] or
    /// [`Self::execute_commands`] to be called, it will deadlock.
    pub fn try_execute_commands(&self, mut execute: impl FnMut(C) -> Result<()>) -> Result<()> {
        let mut commands = self.commands.write();
        while let Some(command) = commands.pop_front() {
            execute(command)?;
        }
        Ok(())
    }

    /// Removes all commands in the queue.
    pub fn clear(&self) {
        self.commands.write().clear();
    }
}

impl<C> Default for CommandQueue<C> {
    fn default() -> Self {
        Self::new()
    }
}
