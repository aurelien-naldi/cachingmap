//! A [LazyMap] adds caching to a mapping closure to call it only once on each key and only if needed.
//!
//! The closure is called only if the key is missing from the cache. Note that only the first call will execute the closure,
//! all following calls with the same key will then use the cache, even if the output of the closure is not stable.
//!
//! ```
//! use cachingmap::LazyMap;
//!
//! // Suppose that we have an expensive function returning predictable results (without side effects)
//! fn compute_value(seed: &usize) -> String // content skipped
//! # {   format!("Computed for seed {}", *seed) }
//! # let (comp1, comp2, comp10) = (compute_value(&1), compute_value(&2), compute_value(&10));
//!
//! // Create a cache and use closures to compute and cache the result
//! let mut cache = LazyMap::new(|k| compute_value(&k));
//! let ref1 = cache.get(1);
//! let ref3 = cache.get(3);
//!
//! // If we call it on an existing key, the closure is NOT executed
//! // and we obtain a reference to the previously cached object
//! let ref1b = cache.get(1,);    // same result, skipped
//!
//! // Only the first inserted a value is stored in the cache. All references then borrow the same data
//! assert!(std::ptr::eq(ref1, ref1b));
//! # assert_eq!(ref1, &comp1);
//! # assert_eq!(ref1b, &comp1);
//! # assert_ne!(ref1b, &comp10);
//!
//! // Any mutable access to the cache invalidates all previously returned references.
//! // This allows to clear the full cache, remove or replace individual entries, ...
//! cache.remove(&1);
//! let ref1c = cache.get(1);
//!
//! // The borrow checker now rejects the use of any previously returned references
//! // (e.g. ref1  is NOT ACCESSIBLE after the call to remove).
//! # assert_eq!(ref1c, &comp1);
//! ```
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;

/// Lazy value mapping based on a closure and a caching map to store.
///
/// Map values are lazily computed using a closure on the first access and
/// then stored in a caching hashmap for later access.
///
/// Uses unsafe code for interior mutability as we need to expose references inside an UnsafeCell.
pub struct LazyMap<K, V> {
    f: fn(K) -> V,
    cache: UnsafeCell<HashMap<K, Box<V>>>,
}

impl<K: Eq + Hash + Copy, V> LazyMap<K, V> {
    /// Create a LazyMap using a closure
    pub fn new(f: fn(K) -> V) -> Self {
        Self {
            f,
            cache: UnsafeCell::default(),
        }
    }

    pub fn get(&self, key: K) -> &V {
        // Get unsafe mutable access to the caching map.
        // This operation is safe as long as references to the existing content remain unmodified.
        // Here it holds as we only add new values and boxing ensure stability of the references.
        unsafe { self.cache.get().as_mut().unwrap() }
            .entry(key)
            .or_insert_with(|| Box::new((self.f)(key)))
    }

    pub fn remove(&mut self, key: &K) -> Option<Box<V>> {
        self.cache.get_mut().remove(key)
    }

    pub fn clear(&mut self) {
        self.cache.get_mut().clear()
    }

    pub fn len(&self) -> usize {
        unsafe { self.cache.get().as_ref().unwrap() }.len()
    }
}
