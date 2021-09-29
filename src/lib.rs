//! This crate defines the [CachingMap] trait: a map accepting insertion of **new** entries while immutable.
//!
//! The most convenient use of the caching map is to request a cached value with a closure to compute it if needed.
//! The closure will be called only if the key is missing from the cache. Note that only the first call will execute
//! the closure, all following calls will use the cache, even if the closure is different or not predictable.
//!
//! ```
//! use cachingmap::*;
//!
//! // Suppose that we have an expensive function returning predictable results
//! fn compute_value(seed: &usize) -> String // content skipped
//! # {   format!("Computed for seed {}", *seed) }
//! # let (comp1, comp2, comp10) = (compute_value(&1), compute_value(&2), compute_value(&10));
//!
//! // Create a cache and use closures to compute and cache the result
//! let mut cache = AsyncCachingMap::default();
//! let ref1 = cache.cache(1, |v| compute_value(v));
//! let ref3 = cache.cache(3, compute_value);
//!
//! // If we call it on an existing key, the closure is NOT executed
//! // and we obtain a reference to the previously cached object
//! let ref1b = cache.cache(1, |v| compute_value(v));    // same result, skipped
//! let ref1c = cache.cache(1, |v| compute_value(&10));   // different result, also skipped
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
//! let ref1d = cache.cache(1, |v| compute_value(&10));
//!
//! // The borrow checker now rejects the use of any previously returned references
//! // ref1.is_new();  // Does NOT compile after the call to remove
//! # assert_eq!(*ref1d, &comp10);
//! ```
use std::borrow::Cow;
use std::ops::Deref;

mod boxed;

pub use boxed::AsyncCachingMap;

/// A Map accepting immutable insertion of new entries.
///
/// See the [crate doc](crate) for a simple example.
///
/// # Finer control of the cache
///
/// If some results can be obtained efficiently and without new allocation, we may want to avoid wasting memory by caching them.
/// The [CachingMap::cache_owned] method takes a closure returning a [Cow] object, it will only cache the Owned results.
pub trait CachingMap<K, V: Clone> {
    /// Retrieve an entry from the cache and use a closure to compute and add it if missing.
    ///
    /// If the key is not in the cache, execute the closure to get the value and add cache it.
    /// return a reference to the cached value (existing or newly added)
    ///
    /// Note that if a cached value exists, the closure is ignored.
    /// Calling this with different closures or closures that do not always return the same value
    /// can give unexpected results.
    fn cache<F: Fn(&K) -> V>(&self, key: K, f: F) -> CachedValue<V> {
        self.cache_owned(key, |k| Cow::Owned(f(k)))
    }

    /// Retrieve an entry from the cache and use a closure to compute and add it if missing
    ///
    /// This is a variant of [Self::cache] with similar properties:
    /// if the key is in the cache,the value in cache will be returned immediately,
    /// otherwise, the closure is used to get a value. In this variant, the value obtained with
    /// the closure is a [Cow] object, which can be either owned or borrowed.
    /// Borrowed values are returned without updating the cache.
    /// Owned values are cached before returning an internal reference.
    fn cache_owned<'a, F: Fn(&K) -> Cow<'a, V>>(&'a self, key: K, f: F) -> CachedValue<'a, V>;
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
