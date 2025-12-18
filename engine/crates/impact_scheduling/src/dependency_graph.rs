//! Dependency graph for task scheduling.

use super::{Task, TaskID};
use anyhow::{Result, bail};
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_containers::{FixedQueue, NoHashMap};
use std::{marker::PhantomData, mem};
use tinyvec::TinyVec;

/// A graph describing the dependencies between separate tasks.
#[derive(Debug)]
pub struct TaskDependencyGraph<S> {
    dependencies: NoHashMap<TaskID, DependencyTaskIDs>,
    dependents: NoHashMap<TaskID, DependentTaskIDs>,
    _phantom: PhantomData<S>,
}

type DependencyTaskIDs = TinyVec<[TaskID; 32]>;
type DependentTaskIDs = TinyVec<[TaskID; 32]>;

impl<S> TaskDependencyGraph<S> {
    pub fn new() -> Self {
        let dependencies = NoHashMap::default();
        let dependents = NoHashMap::default();
        Self {
            dependencies,
            dependents,
            _phantom: PhantomData,
        }
    }

    pub fn add_task(&mut self, task: &impl Task<S>) {
        let task_id = task.id();
        let dependency_ids = task.depends_on();

        if self
            .dependencies
            .insert(task_id, dependency_ids.into())
            .is_some()
        {
            panic!("Task {task_id} added multiple times");
        }

        self.dependents.entry(task_id).or_default();

        for dependency_id in task.depends_on() {
            self.dependents
                .entry(*dependency_id)
                .or_default()
                .push(task_id);
        }
    }

    pub fn obtain_ordered_task_ids<A: Allocator>(&mut self, alloc: A) -> Result<AVec<TaskID, A>> {
        let n_tasks = self.dependencies.len();
        let mut sorted_ids = AVec::with_capacity_in(n_tasks, alloc);

        push_topologically_sorted_task_ids(&self.dependencies, &self.dependents, &mut sorted_ids)?;

        Ok(sorted_ids)
    }

    pub fn find_dependent_task_ids(&self, task_id: TaskID) -> &[TaskID] {
        self.dependents
            .get(&task_id)
            .map(|dependents| dependents.as_slice())
            .unwrap_or_default()
    }
}

fn push_topologically_sorted_task_ids<A: Allocator>(
    dependencies: &NoHashMap<TaskID, DependencyTaskIDs>,
    dependents: &NoHashMap<TaskID, DependentTaskIDs>,
    sorted_task_ids: &mut AVec<TaskID, A>,
) -> Result<()> {
    let n_tasks = dependencies.len();

    let arena = ArenaPool::get_arena_for_capacity(
        n_tasks * (mem::size_of::<usize>() + 2 * mem::size_of::<TaskID>()), // Roughly
    );

    let mut dependency_counts =
        NoHashMap::with_capacity_and_hasher_in(n_tasks, Default::default(), &arena);

    let mut queue = FixedQueue::with_capacity_in(n_tasks, &arena);

    dependency_counts.extend(
        dependencies
            .iter()
            .map(|(&dependent_id, dependencies)| (dependent_id, dependencies.len())),
    );

    // Queue each task with no dependencies
    for (&dependent_id, &dependency_count) in &dependency_counts {
        if dependency_count == 0 {
            queue.push_back(dependent_id);
        }
    }

    // Traverse in topological order to determine evaluation order
    while let Some(task_id) = queue.pop_front() {
        sorted_task_ids.push(task_id);

        for &dependent_id in &dependents[&task_id] {
            // Decrement remaining dependecy count and enqueue when ready
            let count = dependency_counts.get_mut(&dependent_id).unwrap();
            *count -= 1;
            if *count == 0 {
                queue.push_back(dependent_id);
            }
        }
    }

    if sorted_task_ids.len() != n_tasks {
        bail!("Found circular task dependencies");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExecutionTags;
    use anyhow::Result;
    use impact_alloc::Global;
    use impact_containers::HashMap;

    #[derive(Debug)]
    struct TestTask {
        id: TaskID,
        dependencies: Vec<TaskID>,
    }

    impl TestTask {
        fn new(name: &str) -> Self {
            Self {
                id: TaskID::from_str(name),
                dependencies: Vec::new(),
            }
        }

        fn with_dependencies(name: &str, deps: &[&str]) -> Self {
            Self {
                id: TaskID::from_str(name),
                dependencies: deps.iter().map(|&dep| TaskID::from_str(dep)).collect(),
            }
        }
    }

    impl Task<()> for TestTask {
        fn id(&self) -> TaskID {
            self.id
        }

        fn depends_on(&self) -> &[TaskID] {
            &self.dependencies
        }

        fn execute(&self, _external_state: &()) -> Result<(), anyhow::Error> {
            Ok(())
        }

        fn should_execute(&self, _execution_tags: &ExecutionTags) -> bool {
            true
        }
    }

    #[test]
    fn creating_empty_dependency_graph_works() {
        let graph = TaskDependencyGraph::<()>::new();

        assert_eq!(graph.dependencies.len(), 0);
        assert_eq!(graph.dependents.len(), 0);
    }

    #[test]
    fn obtaining_ordered_ids_from_empty_graph_works() {
        let mut graph: TaskDependencyGraph<()> = TaskDependencyGraph::new();

        let ordered_ids = graph.obtain_ordered_task_ids(Global).unwrap();
        assert_eq!(ordered_ids.len(), 0);
    }

    #[test]
    fn adding_single_task_without_dependencies_works() {
        let mut graph = TaskDependencyGraph::new();
        let task = TestTask::new("single_task");

        graph.add_task(&task);

        let ordered_ids = graph.obtain_ordered_task_ids(Global).unwrap();
        assert_eq!(ordered_ids.len(), 1);
        assert_eq!(ordered_ids[0], task.id());
    }

    #[test]
    fn adding_multiple_independent_tasks_works() {
        let mut graph = TaskDependencyGraph::new();
        let task1 = TestTask::new("task1");
        let task2 = TestTask::new("task2");
        let task3 = TestTask::new("task3");

        graph.add_task(&task1);
        graph.add_task(&task2);
        graph.add_task(&task3);

        let ordered_ids = graph.obtain_ordered_task_ids(Global).unwrap();
        assert_eq!(ordered_ids.len(), 3);
        assert!(ordered_ids.contains(&task1.id()));
        assert!(ordered_ids.contains(&task2.id()));
        assert!(ordered_ids.contains(&task3.id()));
    }

    #[test]
    fn adding_tasks_with_simple_dependency_chain_works() {
        let mut graph = TaskDependencyGraph::new();
        let task1 = TestTask::new("task1");
        let task2 = TestTask::with_dependencies("task2", &["task1"]);
        let task3 = TestTask::with_dependencies("task3", &["task2"]);

        graph.add_task(&task1);
        graph.add_task(&task2);
        graph.add_task(&task3);

        let ordered_ids = graph.obtain_ordered_task_ids(Global).unwrap();
        assert_eq!(ordered_ids.len(), 3);

        // Check that dependencies come before dependents
        let pos1 = ordered_ids.iter().position(|&id| id == task1.id()).unwrap();
        let pos2 = ordered_ids.iter().position(|&id| id == task2.id()).unwrap();
        let pos3 = ordered_ids.iter().position(|&id| id == task3.id()).unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    #[test]
    fn adding_tasks_with_diamond_dependency_works() {
        let mut graph = TaskDependencyGraph::new();
        let root = TestTask::new("root");
        let left = TestTask::with_dependencies("left", &["root"]);
        let right = TestTask::with_dependencies("right", &["root"]);
        let merge = TestTask::with_dependencies("merge", &["left", "right"]);

        graph.add_task(&root);
        graph.add_task(&left);
        graph.add_task(&right);
        graph.add_task(&merge);

        let ordered_ids = graph.obtain_ordered_task_ids(Global).unwrap();
        assert_eq!(ordered_ids.len(), 4);

        // Check ordering constraints
        let root_pos = ordered_ids.iter().position(|&id| id == root.id()).unwrap();
        let left_pos = ordered_ids.iter().position(|&id| id == left.id()).unwrap();
        let right_pos = ordered_ids.iter().position(|&id| id == right.id()).unwrap();
        let merge_pos = ordered_ids.iter().position(|&id| id == merge.id()).unwrap();

        assert!(root_pos < left_pos);
        assert!(root_pos < right_pos);
        assert!(left_pos < merge_pos);
        assert!(right_pos < merge_pos);
    }

    #[test]
    fn complex_dependency_graph_ordering_works() {
        let mut graph = TaskDependencyGraph::new();

        // Create a more complex dependency structure
        let a = TestTask::new("A");
        let b = TestTask::with_dependencies("B", &["A"]);
        let c = TestTask::with_dependencies("C", &["A"]);
        let d = TestTask::with_dependencies("D", &["B", "C"]);
        let e = TestTask::with_dependencies("E", &["B"]);
        let f = TestTask::with_dependencies("F", &["D", "E"]);
        let g = TestTask::new("G"); // Independent task

        graph.add_task(&a);
        graph.add_task(&b);
        graph.add_task(&c);
        graph.add_task(&d);
        graph.add_task(&e);
        graph.add_task(&f);
        graph.add_task(&g);

        let ordered_ids = graph.obtain_ordered_task_ids(Global).unwrap();
        assert_eq!(ordered_ids.len(), 7);

        // Verify ordering constraints
        let positions: HashMap<TaskID, usize> = ordered_ids
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        assert!(positions[&a.id()] < positions[&b.id()]);
        assert!(positions[&a.id()] < positions[&c.id()]);
        assert!(positions[&b.id()] < positions[&d.id()]);
        assert!(positions[&c.id()] < positions[&d.id()]);
        assert!(positions[&b.id()] < positions[&e.id()]);
        assert!(positions[&d.id()] < positions[&f.id()]);
        assert!(positions[&e.id()] < positions[&f.id()]);
    }

    #[test]
    #[should_panic]
    fn adding_same_task_twice_panics() {
        let mut graph = TaskDependencyGraph::new();
        let task = TestTask::new("duplicate_task");

        graph.add_task(&task);
        graph.add_task(&task); // Should panic
    }

    #[test]
    fn circular_dependency_detection_works() {
        let mut graph = TaskDependencyGraph::new();
        let task1 = TestTask::with_dependencies("task1", &["task2"]);
        let task2 = TestTask::with_dependencies("task2", &["task1"]);

        graph.add_task(&task1);
        graph.add_task(&task2);

        let result = graph.obtain_ordered_task_ids(Global);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[test]
    fn complex_circular_dependency_detection_works() {
        let mut graph = TaskDependencyGraph::new();
        let task1 = TestTask::with_dependencies("task1", &["task3"]);
        let task2 = TestTask::with_dependencies("task2", &["task1"]);
        let task3 = TestTask::with_dependencies("task3", &["task2"]);

        graph.add_task(&task1);
        graph.add_task(&task2);
        graph.add_task(&task3);

        let result = graph.obtain_ordered_task_ids(Global);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[test]
    fn finding_dependent_task_ids_for_nonexistent_task_works() {
        let graph: TaskDependencyGraph<()> = TaskDependencyGraph::new();
        let nonexistent_id = TaskID::from_str("nonexistent");

        let dependents = graph.find_dependent_task_ids(nonexistent_id);
        assert_eq!(dependents.len(), 0);
    }

    #[test]
    fn finding_dependent_task_ids_with_no_dependents_works() {
        let mut graph = TaskDependencyGraph::new();
        let task = TestTask::new("isolated_task");

        graph.add_task(&task);

        let dependents = graph.find_dependent_task_ids(task.id());
        assert_eq!(dependents.len(), 0);
    }

    #[test]
    fn finding_dependent_task_ids_with_single_dependent_works() {
        let mut graph = TaskDependencyGraph::new();
        let task1 = TestTask::new("task1");
        let task2 = TestTask::with_dependencies("task2", &["task1"]);

        graph.add_task(&task1);
        graph.add_task(&task2);

        let dependents = graph.find_dependent_task_ids(task1.id());
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], task2.id());
    }

    #[test]
    fn finding_dependent_task_ids_with_multiple_dependents_works() {
        let mut graph = TaskDependencyGraph::new();
        let root = TestTask::new("root");
        let dep1 = TestTask::with_dependencies("dep1", &["root"]);
        let dep2 = TestTask::with_dependencies("dep2", &["root"]);
        let dep3 = TestTask::with_dependencies("dep3", &["root"]);

        graph.add_task(&root);
        graph.add_task(&dep1);
        graph.add_task(&dep2);
        graph.add_task(&dep3);

        let dependents = graph.find_dependent_task_ids(root.id());
        assert_eq!(dependents.len(), 3);
        assert!(dependents.contains(&dep1.id()));
        assert!(dependents.contains(&dep2.id()));
        assert!(dependents.contains(&dep3.id()));
    }
}
