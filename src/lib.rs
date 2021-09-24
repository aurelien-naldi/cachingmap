//! This crate provides the [CachingMap] struct, which is a [HashMap] accepting insertion of **new** entries while immutable.
//!
//! It relies on a regular [HashMap] hidden behind an [UnsafeCell] and accessible through [Deref] and [DerefMut].
//! On top of this, the [CachingMap::cache] allows to insert a new entry in an immutable CachingMap.
//! Safety is maintained by refusing to replace existing entries: all previously returned references remain valid
//! as long as only immutable methods are used.
//!
//! > ⚠️ [CachingMap] is **NOT thread safe** and should be rejected by the compiler.
//!
//! # Get from cache or compute only if needed
//!
//! The most convenient use of the caching map is to request a cached value with a closure to compute it if needed.
//! The closure will be called only if the key is missing from the cache. Note that only the first call will execute
//! the closure, all following calls will use the cache, even if the closure is different or not predictable.
//! See [CachingMap] doc for other uses.
//!
//! ```
//! use cachingmap::CachingMap;
//!
//! // Suppose that we have an expensive function returning predictable results
//! fn compute_value(seed: &usize) -> String // content skipped
//! # {   format!("Computed for seed {}", *seed) }
//! # let (comp1, comp2, comp10) = (compute_value(&1), compute_value(&2), compute_value(&10));
//!
//! // Create a cache and use closures to compute and cache the result
//! let mut cache = CachingMap::new();
//! let ref1 = cache.cached(1, &|v| compute_value(v));
//!
//! // If we call it on an existing key, the closure is NOT executed
//! // and we obtain a reference to the previously cached object
//! let ref1b = cache.cached(1, &|v| compute_value(v));    // same result, skipped
//! let ref1c = cache.cached(1, &|v| compute_value(&10));   // different result, also skipped
//!
//! // Only the first inserted a value in the cache. All references borrow the same data
//! assert!(ref1.is_new() && ref1b.is_old() && ref1c.is_old());
//! assert!(std::ptr::eq(*ref1, *ref1c));
//! # assert_eq!(*ref1, &comp1);
//! # assert_eq!(*ref1b, &comp1);
//! # assert_ne!(*ref1b, &comp10);
//!
//! // Any mutable access to the cache invalidates previous references.
//! // This allows to clear the full cache, remove or replace individual entries, ...
//! cache.remove(&1);
//! let ref1d = cache.cached(1, &|v| compute_value(&10));
//!
//! // The borrow checker now rejects the use of any previously returned references
//! // ref1.is_new();  // Does NOT compile after the call to remove
//! # assert_eq!(*ref1d, &comp10);
//! ```
use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

/// A HashMap accepting immutable insertion of new entries.
///
/// The internal [HashMap] is available through [Deref] and [DerefMut].
/// New entries can be added on immutable map using the [Self::cache] method.
/// See the [crate doc](crate) for a simple example.
///
/// # Finer control of the cache
///
/// If some results can be obtained efficiently and without new allocation, we may want to avoid wasting memory by caching them.
/// The [CachingMap::cached_cow] method takes a closure returning a [Cow] object, it will only cache the Owned results.
///
/// The high level [CachingMap::cached] and [CachingMap::cached_cow] methods are both built on the [CachingMap::cache] method
/// to add owned objects to the cache explicitly.
///
/// ```
/// use cachingmap::{CachingMap, CachedValue};
///
/// // Create a new cache, it returns None for any key
/// let mut cache = CachingMap::new();
/// # assert!(cache.get(&3).is_none());
///
/// // Manually add a value to the cache
/// let ref1 = cache.cache(1, String::from("something"));
/// assert!(ref1.is_new());
/// assert_eq!(*ref1, "something"); // ref1 is a new reference to the cached String
///
/// // Remember that caching another values for the same key does not change it
/// let ref2 = cache.cache(1, String::from("something else"));
/// assert!(ref2.is_old());
/// assert_eq!(*ref2, "something"); // ref2 is a reference to the original String
/// ```
///
/// Note that as this structure is meant for caching, cloning a CachingMap is equivalent to creating a new one: the content is not copied.
///
/// # Thread safety
///
/// The Caching Map is **NOT** thread safe: threads could share immutable pointers and try to add the same entry.
/// In this case, one thread could return a reference just before the entry is overwritten by the other thread.
#[derive(Debug)]
pub struct CachingMap<K, V> {
    cache: UnsafeCell<HashMap<K, Box<V>>>,
}

/// A reference provided by the cache
///
/// The variants are used to reflect the state of the cache when it has been obtained.
/// All variants carry a reference to the same type of object, available through [Deref].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CachedValue<'a, T> {
    /// The reference has been forwarded from another owner
    Ext(&'a T),
    /// The reference has just been added to the cache
    New(&'a T),
    /// The reference was already in the cache
    Old(&'a T),
}

impl<T> CachedValue<'_, T> {
    pub const fn is_ext(&self) -> bool {
        matches!(*self, Self::Ext(_))
    }
    pub const fn is_new(&self) -> bool {
        matches!(*self, Self::New(_))
    }
    pub const fn is_old(&self) -> bool {
        matches!(*self, Self::Old(_))
    }
}

impl<'a, T> Deref for CachedValue<'a, T> {
    type Target = &'a T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Ext(v) => v,
            Self::New(v) => v,
            Self::Old(v) => v,
        }
    }
}

impl<K, V> Clone for CachingMap<K, V> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<K, V> Default for CachingMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Deref for CachingMap<K, V> {
    type Target = HashMap<K, Box<V>>;

    fn deref(&self) -> &Self::Target {
        let ptr = self.cache.get();
        unsafe {
            // We never delete or replace the inner hashmap, so we are sure it is available
            ptr.as_ref().unwrap()
        }
    }
}

impl<K, V> DerefMut for CachingMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cache.get_mut()
    }
}

impl<K, V> CachingMap<K, V> {
    pub fn new() -> Self {
        Self {
            cache: UnsafeCell::new(HashMap::new()),
        }
    }
}

impl<K: Hash + Eq + Copy, V: Sized + Clone> CachingMap<K, V> {
    /// Insert an entry in the cache if it was not already there
    pub fn cache(&self, key: K, value: V) -> CachedValue<V> {
        match self.get(&key) {
            Some(v) => CachedValue::Old(v),
            None => unsafe {
                // Adding a new entry to the map is safe: old references remain valid
                let map = self.cache.get().as_mut().unwrap();
                map.insert(key, Box::from(value));
                CachedValue::New(&map[&key])
            },
        }
    }

    /// Retrieve an entry from the cache and use a closure to compute and add it if missing.
    ///
    /// If the key is not in the cache, execute the closure to get the value and add cache it.
    /// return a reference to the cached value (existing or newly added)
    ///
    /// Note that if a cached value exists, the closure is ignored.
    /// Calling this with different closures or closures that do not always return the same value
    /// can give unexpected results.
    pub fn cached(&self, key: K, f: &dyn Fn(&K) -> V) -> CachedValue<V> {
        match self.get(&key) {
            Some(value) => CachedValue::Old(value),
            None => self.cache(key, f(&key)),
        }
    }

    /// Retrieve an entry from the cache and use a closure to compute and add it if missing
    ///
    /// This is a variant of [Self::cached] with similar properties:
    /// if the key is in the cache,the value in cache will be returned immediately,
    /// otherwise, the closure is used to get a value. In this variant, the value obtained with
    /// the closure is a [Cow] object, which can be either owned or borrowed.
    /// Borrowed values are returned without updating the cache.
    /// Owned values are cached before returning an internal reference.
    pub fn cached_cow<'a>(&'a self, key: K, f: &dyn Fn(&K) -> Cow<'a, V>) -> CachedValue<'a, V> {
        match self.get(&key) {
            Some(value) => CachedValue::Old(value),
            None => match f(&key) {
                Cow::Borrowed(value) => CachedValue::Ext(value),
                Cow::Owned(value) => self.cache(key, value),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use crate::CachingMap;

    #[test]
    fn test_stability() {
        let cache = CachingMap::new();
        let key = 12523;
        let first = cache.cache(key, 25);
        for i in 1..10000 {
            cache.cache(i, 5 * i + 8);
        }
        let after = cache.cache(key, 1);

        assert!(first.is_new());
        assert!(after.is_old());
        assert!(std::ptr::eq(*first, *after));
    }
}
