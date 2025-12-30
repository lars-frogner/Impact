//! Macros for scheduling.

/// Macro for defining a new empty type that implements the
/// [`Task`](crate::Task) trait.
///
/// The macro takes as input the name of the new task type, the other tasks
/// (also defined with this macro) this task depends on, the execution tags
/// (defined with the `define_execution_tag` macro) that should trigger this
/// task, and a closure that takes a reference to some state object and executes
/// the task on it.
///
/// # Examples
/// ```no_run
/// # use impact_scheduling::{define_execution_tag, define_task, TaskScheduler};
/// # use std::{num::NonZeroUsize, sync::Arc};
/// #
/// # #[derive(Clone)]
/// # struct Engine;
/// #
/// # impl Engine {
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
///     |engine: &Engine| {
///         engine.update_trajectories();
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
///     |engine: &Engine| {
///         engine.compute_forces();
///         Ok(())
///     }
/// );
///
/// // Define the tag that will trigger execution of the tasks
/// define_execution_tag!(Physics);
///
/// let engine = Engine::new();
/// let n_workers = NonZeroUsize::new(2).unwrap();
/// let queue_capacity = NonZeroUsize::new(256).unwrap();
///
/// let mut scheduler = TaskScheduler::new(n_workers, queue_capacity, engine);
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
        |$state:ident: &$state_ty:ty| $execute:expr
    ) => {
        $(#[$attributes])*
        #[derive(Copy, Clone, Debug)]
        $($pub)? struct $name;

        impl $name {
            $($pub)? const TASK_ID: ::impact_scheduling::TaskID = ::impact_scheduling::TaskID::new(stringify!($name));

            const N_DEPENDENCIES: usize = $crate::count_ident_args!($($dep),*);
            const DEPENDENCY_IDS: [::impact_scheduling::TaskID; Self::N_DEPENDENCIES] = [$($dep::TASK_ID),*];

            const N_EXECUTION_TAGS: usize = $crate::count_ident_args!($($tag),*);
            const EXECUTION_TAGS: [$crate::ExecutionTag; Self::N_EXECUTION_TAGS] = [$($tag::EXECUTION_TAG),*];
        }

        impl $crate::Task<$state_ty> for $name {
            fn id(&self) -> ::impact_scheduling::TaskID {
                Self::TASK_ID
            }

            fn depends_on(&self) -> &[::impact_scheduling::TaskID] {
                &Self::DEPENDENCY_IDS
            }

            fn execute(&self, $state: &$state_ty) -> anyhow::Result<()> {
                $execute
            }

            fn should_execute(&self, execution_tags: &$crate::ExecutionTags) -> bool {
                Self::EXECUTION_TAGS.iter().any(|tag| execution_tags.contains(tag))
            }
        }
    };
}

/// Macro for defining a new empty type representing an
/// [`ExecutionTag`](crate::ExecutionTag), for use in the [`define_task`] macro.
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
            $($pub)? const EXECUTION_TAG: $crate::ExecutionTag = $crate::ExecutionTag::new(stringify!($name));
        }
    };
}

/// Macro that creates a static `Arc<ExecutionTags>`
/// variable with the given name containing the given list of
/// execution tags (defined with the `define_execution_tag`
/// macro).
#[macro_export]
macro_rules! define_execution_tag_set {
    (
        $([$pub:ident])? $name:ident, [$($tag:ident),*]
    ) => {
        $($pub)? static $name: ::std::sync::LazyLock<::std::sync::Arc<$crate::ExecutionTags>> = ::std::sync::LazyLock::new(|| ::std::sync::Arc::new($crate::ExecutionTags::from_iter([$($tag::EXECUTION_TAG),*])));
    };
}

/// This macro expands to a compile time constant equal
/// to the number of arguments passed to the macro.
#[doc(hidden)]
#[macro_export]
macro_rules! count_ident_args {
    ($($arg:ident),*) => {
        // Ugly hack utilizing that `[]::len` is a `const fn`
        // (the extra "" and -1 are needed for the hack to work for zero args)
        ["", $(stringify!($arg)),*].len() - 1
    };
}
