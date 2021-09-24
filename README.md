# Caching map

This crate provides a map designed for caching intermediate results by accepting new entries while immutable.
It provides simple immutable references to the cached values which remain valid while new entries are added.
To return lightweight references without a guard or any encapsulation, it uses an UnsafeCell and short pieces
of unsafe code. The returned references are guaranted to remain safe as existing entries can **NOT** be removed
without proper mutable access. Values are boxed to ensure that pointers remain stable when the HashMap is extended.

> ⚠️  Earlier versions did not use boxed values, old returned references could become invalid.

> ⚠️  This map is **NOT thread-safe**, but may be accepted in multi-threaded code (untested).

The main objective is to include this map in a larger data structure and to use it to store the result of some
computations based on other fields. Individual entries may then need to be cleared when some fields are modified
to ensure that the next returned result remain valid.


The current version is a proof of concept, with some potential room for improvement.
* The code is short and seems valid in single-threaded context, but uses unsafe code
* It is probably not thread safe. If it compiles in multi-threaded context, it should be blocked.
  A thread-safe extension could be useful.
* It uses an internal HashMap, but could be extended to other Indexable backend.


