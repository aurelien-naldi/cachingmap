//! This crate provides the [CachingMap] struct, which is a [HashMap] accepting insertion of **new** entries while immutable.
//!
//! It relies on a regular [HashMap] hidden behind an [UnsafeCell] and accessible through [Deref] and [DerefMut].
//! On top of this, the [CachingMap::cache] allows to insert a new entry in an immutable CachingMap.
//! Safety is maintained by refusing to replace existing entries: all previously returned references remain valid
//! as long as only immutable methods are used.
//!
//! > ⚠️ This map is **NOT thread safe**, beware that the use of unsafe may fool the borrow checker (not tested).
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
//! fn compute_value(seed: usize) -> String // content skipped
//! # {   format!("Computed for seed {}", seed) }
//! # let (comp1, comp2, comp10) = (compute_value(1), compute_value(2), compute_value(10));
//!
//! // Create a cache and use closures to compute and cache the result
//! let mut cache = CachingMap::new();
//! let ref1 = cache.get_or_cache(1, &|| compute_value(1));
//!
//! // If we call it on an existing key, the closure is **not** executed
//! // and we obtain a reference to the previously cached object
//! let ref1b = cache.get_or_cache(1, &|| compute_value(1));    // same result, skipped
//! let ref1c = cache.get_or_cache(1, &|| compute_value(10));   // different result, also skipped
//! assert_eq!(ref1, ref1c);
//! # assert_eq!(ref1, &comp1);
//! # assert_eq!(ref1b, &comp1);
//! # assert_ne!(ref1b, &comp10);
//!
//! // Any mutable access to the cache invalidates previous references.
//! // This allows to clear the full cache, remove or replace individual entries, ...
//! cache.remove(&1);
//! let ref1d = cache.get_or_cache(1, &|| compute_value(10));
//! // The borrow checker now rejects the use of any previously returned references
//! // println!("{}", ref1);  // Does NOT compile after the call to remove
//!
//! # assert_ne!(ref1d, &comp1);
//! # assert_eq!(ref1d, &comp10);
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
/// The [CachingMap::get_or_cache_cow] method takes a closure returning a [Cow] object, it will only cache the Owned results.
///
/// The high level [CachingMap::get_or_cache] and [CachingMap::get_or_cache_cow] methods are both built on the [CachingMap::cache] method
/// to add owned objects to the cache explicitly.
///
/// ```
/// use cachingmap::CachingMap;
///
/// // Create a new cache, it returns None for any key
/// let mut cache = CachingMap::new();
/// # assert!(cache.get(&3).is_none());
///
/// // Manually add a value to the cache
/// let (ref1,b) = cache.cache(1, String::from("something"));
/// assert!(b);                  // b is true as the value has been added to the cache
/// assert_eq!(ref1, "something"); // ref1 is a reference to the cached String
///
/// // Remember that caching another values for the same key does not change it
/// let (ref2,b) = cache.cache(1, String::from("something else"));
/// assert!(!b);                 // b is false as the cache already had this key
/// assert_eq!(ref2, "something"); // ref2 is a reference to the original String
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
    cache: UnsafeCell<HashMap<K, V>>,
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
    type Target = HashMap<K, V>;

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
    pub fn cache(&self, key: K, value: V) -> (&V, bool) {
        match self.get(&key) {
            None => {
                unsafe {
                    // Adding a new entry to the map is safe: old references remain valid
                    let map = self.cache.get().as_mut().unwrap();
                    map.insert(key, value);
                    (map.get(&key).unwrap(), true)
                }
            }
            Some(v) => (v, false),
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
    pub fn get_or_cache(&self, key: K, f: &dyn Fn() -> V) -> &V {
        match self.get(&key) {
            Some(value) => value,
            None => {
                self.cache(key, f());
                self.get(&key).unwrap()
            }
        }
    }

    /// Retrieve an entry from the cache and use a closure to compute and add it if missing
    ///
    /// This is a variant of [Self::get_or_cache] with similar properties:
    /// if the key is in the cache,the value in cache will be returned immediately,
    /// otherwise, the closure is used to get a value. In this variant, the value obtained with
    /// the closure is a [Cow] object, which can be either owned or borrowed.
    /// Borrowed values are returned without updating the cache.
    /// Owned values are cached before returning an internal reference.
    pub fn get_or_cache_cow<'a>(&'a self, key: K, f: &dyn Fn() -> Cow<'a, V>) -> &'a V {
        match self.get(&key) {
            Some(value) => value,
            None => match f() {
                Cow::Borrowed(value) => value,
                Cow::Owned(value) => {
                    self.cache(key, value);
                    self.get(&key).unwrap()
                }
            },
        }
    }
}
