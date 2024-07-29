use alloc::{boxed::Box, vec::Vec};
use core::hash::{BuildHasher, Hash};

use fnv::FnvBuildHasher;

#[expect(clippy::module_name_repetitions)]
pub struct DirectMappedCache<T: Hash + PartialEq, S: BuildHasher = FnvBuildHasher> {
    cache: Box<[Option<T>]>,
    build_hasher: S,
}

impl<T: Hash + PartialEq> DirectMappedCache<T, FnvBuildHasher> {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, FnvBuildHasher::default())
    }
}

impl<T: Hash + PartialEq, S: BuildHasher> DirectMappedCache<T, S> {
    #[must_use]
    pub fn with_capacity_and_hasher(capacity: usize, build_hasher: S) -> Self {
        let mut cache = Vec::with_capacity(capacity);
        cache.resize_with(capacity, || None);

        Self {
            cache: cache.into_boxed_slice(),
            build_hasher,
        }
    }
}

impl<T: Hash + PartialEq, B: BuildHasher> DirectMappedCache<T, B> {
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    #[must_use]
    pub fn insert(&mut self, value: T) -> bool {
        if self.capacity() == 0 {
            return true;
        }

        let hash = self.build_hasher.hash_one(&value);

        #[expect(clippy::cast_possible_truncation)]
        let index = (hash % (self.capacity() as u64)) as usize;

        let bucket = &mut self.cache[index];
        let insert = bucket.as_ref().map_or(true, |old| old != &value);
        *bucket = Some(value);

        insert
    }
}
