use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use crate::*;

/// Simple caching map using UnsafeCell and boxed values.
///
/// It relies on a regular [HashMap] hidden behind an [UnsafeCell] and accessible through [Deref] and [DerefMut].
/// Safety is maintained by refusing to replace existing entries: all previously returned references remain valid
/// as long as only immutable methods are used. The values are boxed to ensure that raw pointers remain valid
/// despite the shuffling if hashmap entries when extending the allocated memory.
///
/// This implementation of CachingMap is **NOT thread safe**.
#[derive(Debug)]
pub struct AsyncCachingMap<K, V> {
    cache: UnsafeCell<HashMap<K, Box<V>>>,
    full_clone: bool,
}

impl<K, V> AsyncCachingMap<K, V> {
    /// Create a new caching map.
    ///
    /// Equivalent to [```with_full_clone(true)```](Self::with_full_clone).
    pub fn new() -> Self {
        Self::with_full_clone(true)
    }

    /// Create a new caching map with a choice of cloning strategy.
    ///
    /// If ```full_clone``` is true (by default), the clone operation will clone the inner hashmap.
    /// Otherwise it will create a new empty hashmap, which may be a better behaviour for a cache.
    ///
    /// Full cloning is enabled by default as it corresponds to the expected behaviour.
    /// Disabling full cloning can however be convenient in a caching context.
    pub fn with_full_clone(full_clone: bool) -> Self {
        Self {
            cache: UnsafeCell::new(HashMap::new()),
            full_clone,
        }
    }

    pub fn full_clone(&self) -> bool {
        self.full_clone
    }

    pub fn set_full_clone(&mut self, full_clone: bool) {
        self.full_clone = full_clone;
    }
}

impl<K: Clone, V: Clone> Clone for AsyncCachingMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            cache: UnsafeCell::new(if self.full_clone {
                self.deref().clone()
            } else {
                HashMap::new()
            }),
            full_clone: self.full_clone,
        }
    }
}

impl<K, V> Default for AsyncCachingMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Deref for AsyncCachingMap<K, V> {
    type Target = HashMap<K, Box<V>>;

    fn deref(&self) -> &Self::Target {
        let ptr = self.cache.get();
        unsafe {
            // Immutable access to the inner map is safe while self is borrowed
            ptr.as_ref().unwrap()
        }
    }
}

impl<K, V> DerefMut for AsyncCachingMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cache.get_mut()
    }
}

impl<K: Hash + Eq + Copy, V: Sized + Clone> crate::CachingMap<K, V> for AsyncCachingMap<K, V> {
    fn cache_owned<'a, F: Fn(&K) -> Cow<'a, V>>(&'a self, key: K, f: F) -> CachedValue<'a, V> {
        match self.get(&key) {
            Some(value) => CachedValue::Old(value),
            None => match f(&key) {
                Cow::Borrowed(value) => CachedValue::Ext(value),
                Cow::Owned(value) => unsafe {
                    // Adding a new entry to the map is safe: old references remain valid
                    let map = self.cache.get().as_mut().unwrap();
                    map.insert(key, Box::new(value));
                    CachedValue::New(&map[&key])
                },
            },
        }
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_stability() {
        let cache = AsyncCachingMap::<usize, usize>::default();
        let key = 12523;
        let first = cache.cache(key, |_| 25);
        for i in 1..10000 {
            cache.cache(i, |i| 5 * i + 8);
        }
        let after = cache.cache(key, |_| 1);

        assert!(first.is_new());
        assert!(after.is_old());
        assert!(std::ptr::eq(*first, *after));
    }
}
