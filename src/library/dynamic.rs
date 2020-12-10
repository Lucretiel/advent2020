//! A library for doing dynamic programming in a non-recursive way

use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

pub trait SubtaskStore<K, V> {
    /// Add a new subtask solution to the store. Return the old solution, if
    /// present.
    fn add(&mut self, goal: K, solution: V) -> Option<V>;

    /// Fetch a known solution for a subtask, if possible
    fn get(&self, goal: &K) -> Option<&V>;

    /// Check if a subtask has a known solution
    fn contains(&self, goal: &K) -> bool;
}

impl<K, V, S> SubtaskStore<K, V> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: Default + BuildHasher,
{
    fn add(&mut self, goal: K, solution: V) -> Option<V> {
        self.insert(goal, solution)
    }

    fn get(&self, goal: &K) -> Option<&V> {
        self.get(goal)
    }

    fn contains(&self, goal: &K) -> bool {
        self.contains_key(goal)
    }
}

impl<K: Ord, V> SubtaskStore<K, V> for BTreeMap<K, V> {
    fn add(&mut self, goal: K, solution: V) -> Option<V> {
        self.insert(goal, solution)
    }

    fn get<'a>(&'a self, goal: &K) -> Option<&V> {
        self.get(goal)
    }

    fn contains(&self, goal: &K) -> bool {
        self.contains_key(goal)
    }
}

#[derive(Debug)]
pub struct Dependency<'a, K> {
    key: K,
    lifetime: PhantomData<&'a K>,
}

#[derive(Debug)]
pub enum TaskInterrupt<'a, K, E> {
    Dependency(Dependency<'a, K>),
    Error(E),
}

impl<'a, K, E> From<Dependency<'a, K>> for TaskInterrupt<'a, K, E> {
    fn from(dep: Dependency<'a, K>) -> Self {
        TaskInterrupt::Dependency(dep)
    }
}

pub trait Subtask<K, V> {
    fn precheck(&self, goals: impl IntoIterator<Item = K>) -> Result<(), Dependency<K>>;
    fn solve<'a>(&self, goal: K) -> Result<&V, Dependency<K>>;
}

pub trait Task<K, V, E> {
    fn solve<'sub, T>(&self, goal: &K, subtasker: &'sub T) -> Result<V, TaskInterrupt<'sub, K, E>>
    where
        T: Subtask<K, V>;

    fn solve_all<S: SubtaskStore<K, V> + Default>(&self, goal: K) -> Result<V, DynamicError<K, E>>
    where
        Self: Sized,
        K: PartialEq,
    {
        execute(goal, self, S::default())
    }
}

#[derive(Debug)]
pub enum DynamicError<K, E> {
    /// The solver found a circular dependency while solving
    CircularDependency(K),

    /// The solver itself returned an error
    Error(E),
}

impl<K: Debug, E> Display for DynamicError<K, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            DynamicError::CircularDependency(ref dep) => {
                write!(f, "goal {:?} has a circular dependency on itself", dep)
            }
            DynamicError::Error(..) => write!(f, "solver encountered an error"),
        }
    }
}

impl<K: Debug, E: Error + 'static> Error for DynamicError<K, E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            DynamicError::CircularDependency(..) => None,
            DynamicError::Error(ref err) => Some(err),
        }
    }
}

#[derive(Debug, Default)]
struct Subtasker<S> {
    store: S,
}

impl<'a, K, V, S> Subtask<K, V> for Subtasker<S>
where
    S: SubtaskStore<K, V>,
{
    fn precheck(&self, goals: impl IntoIterator<Item = K>) -> Result<(), Dependency<K>> {
        goals
            .into_iter()
            .try_for_each(|goal| match self.store.contains(&goal) {
                true => Ok(()),
                false => Err(Dependency {
                    key: goal,
                    lifetime: PhantomData,
                }),
            })
    }

    fn solve(&self, goal: K) -> Result<&V, Dependency<K>> {
        self.store.get(&goal).ok_or(Dependency {
            key: goal,
            lifetime: PhantomData,
        })
    }
}

/// Solve a dynamic algorithm.
///
/// This will run task.solve(&goal, subtasker). The task can request subgoal
/// solutions by calling `subtasker.solve(subgoal)?`; this will halt the
/// function and call task.solve(&subgoal, subtasker). In this way, execute
/// performs a depth-first traversal of the problem space. Solutions to subtasks
/// are stored in the store and are provided by the subtasker to the caller
/// when available; this ensures that each subtask is solved at most once.
///
/// Note that every time a subtask is requested but not available, the ? will
/// return a dependency request from the solver. This means the solver will be
/// restarted from scratch once for each dependency it requests, until the
/// store can fulfill them all. To prevent wasting work finding a partial
/// solution, you can call `subtasker.precheck(iter)?` at the beginning of
/// your Task::solve implementation with an iterator over all the subgoal
/// dependencies you're expecting
pub fn execute<K: PartialEq, V, E, T: Task<K, V, E>, S: SubtaskStore<K, V>>(
    goal: K,
    task: &T,
    store: S,
) -> Result<V, DynamicError<K, E>> {
    let mut subtasker = Subtasker { store };

    // TODO: use an ordered hash map for faster circular checks
    let mut dependency_stack = vec![];
    let mut current_goal = goal;

    loop {
        // NOTE: We could check if the current_goal is already in the store,
        // but it should be impossible for that to be the case at this point,
        // since the only way to add things to the store is with a Dependency,
        // and the only way to get a Dependency is if the store reports that
        // it *doesn't* already contain that solution.
        //
        // This means that the only time this could happen is if the store
        // contains the solution for the *original* goal, which we assume
        // doesn't happen.

        match task.solve(&current_goal, &subtasker) {
            Ok(solution) => match dependency_stack.pop() {
                None => break Ok(solution),
                Some(dependent_goal) => {
                    subtasker.store.add(current_goal, solution);
                    current_goal = dependent_goal;
                }
            },
            Err(TaskInterrupt::Error(err)) => break Err(DynamicError::Error(err)),
            Err(TaskInterrupt::Dependency(Dependency { key: subgoal, .. })) => {
                dependency_stack.push(current_goal);
                match dependency_stack.contains(&subgoal) {
                    true => break Err(DynamicError::CircularDependency(subgoal)),
                    false => current_goal = subgoal,
                }
            }
        }
    }
}
