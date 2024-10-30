use std::{
    sync::{Arc, Barrier, RwLock},
    task::Poll,
};

use bit_set::BitSet;

#[derive(Clone)]
pub struct Vote<T: Clone> {
    shared: Arc<SharedVote<T>>,
    generation: Generation,
}

struct SharedVote<T: Clone> {
    data: RwLock<GenerationalData<T>>,
    barrier: Barrier,
}

struct GenerationalData<T> {
    data: T,
    generation: Generation,
}

impl<T: Clone> Vote<T> {
    #[must_use]
    pub fn new(n: usize) -> Self
    where
        T: Default,
    {
        Self::new_with_dummy(n, T::default())
    }

    #[must_use]
    pub fn new_with_dummy(n: usize, dummy: T) -> Self {
        Self {
            shared: Arc::new(SharedVote {
                data: RwLock::new(GenerationalData {
                    data: dummy,
                    generation: Generation::first(),
                }),
                barrier: Barrier::new(n),
            }),
            generation: Generation::first().next(),
        }
    }

    pub fn vote(&mut self, vote: impl FnOnce(Option<&T>) -> T) -> T {
        {
            let mut generational_data = self.shared.data.write().unwrap();

            if generational_data.generation == self.generation {
                generational_data.data = vote(Some(&generational_data.data));
            } else {
                generational_data.data = vote(None);
                generational_data.generation = self.generation;
            }
        }

        self.shared.barrier.wait();

        let result = {
            let data = self.shared.data.read().unwrap();
            data.data.clone()
        };

        self.generation = self.generation.next();

        self.shared.barrier.wait();

        result
    }
}

#[expect(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct AsyncVote<T: Clone> {
    shared: Arc<SharedAsyncVote<T>>,
    generation: Generation,
}

struct SharedAsyncVote<T: Clone> {
    data: RwLock<AsyncGenerationalData<T>>,
    barrier: Barrier,
    n: usize,
}

struct AsyncGenerationalData<T> {
    data: T,
    generation: Generation,
    submissions: BitSet,
}

impl<T: Clone> AsyncVote<T> {
    #[expect(dead_code)] // FIXME
    #[must_use]
    pub fn new(n: usize) -> Self
    where
        T: Default,
    {
        Self::new_with_dummy(n, T::default())
    }

    #[must_use]
    pub fn new_with_dummy(n: usize, dummy: T) -> Self {
        Self {
            shared: Arc::new(SharedAsyncVote {
                data: RwLock::new(AsyncGenerationalData {
                    data: dummy,
                    generation: Generation::first(),
                    submissions: BitSet::with_capacity(n),
                }),
                barrier: Barrier::new(n),
                n,
            }),
            generation: Generation::first().next(),
        }
    }

    pub fn vote(&mut self, vote: impl FnOnce(Option<&T>) -> T, rank: u32) -> Poll<T> {
        {
            let mut generational_data = self.shared.data.write().unwrap();

            if generational_data.generation != self.generation {
                // restart the vote with the next generation
                generational_data.data = vote(None);
                generational_data.generation = self.generation;
                assert!(generational_data.submissions.is_empty());
                generational_data.submissions.insert(rank as usize);
            } else if !generational_data.submissions.insert(rank as usize) {
                // first submission for this rank
                generational_data.data = vote(Some(&generational_data.data));
            } else {
                // re-submission for this ranke, no-op
            }

            if generational_data.submissions.len() < self.shared.n {
                return Poll::Pending;
            }
        }

        let wait = self.shared.barrier.wait();

        let result = if wait.is_leader() {
            let mut data = self.shared.data.write().unwrap();
            data.submissions.clear();
            data.data.clone()
        } else {
            let data = self.shared.data.read().unwrap();
            data.data.clone()
        };

        self.generation = self.generation.next();

        self.shared.barrier.wait();

        Poll::Ready(result)
    }

    #[must_use]
    pub fn is_ongoing(&self) -> bool {
        let data: std::sync::RwLockReadGuard<AsyncGenerationalData<T>> =
            self.shared.data.read().unwrap();
        !data.submissions.is_empty()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct Generation(bool);

impl Generation {
    #[must_use]
    pub fn first() -> Self {
        Self(false)
    }

    #[must_use]
    pub fn next(self) -> Self {
        Self(!self.0)
    }
}
