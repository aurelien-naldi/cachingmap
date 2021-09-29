# Caching map

This crate provides a map designed for caching intermediate results by accepting new entries while immutable.
It provides simple immutable references to the cached values which remain valid while new entries are added.

To return lightweight references without a guard or any encapsulation, it uses an UnsafeCell and short pieces
of unsafe code. The returned references are guaranted to remain safe as existing entries can **NOT** be removed
without proper mutable access. Values are boxed to ensure that pointers remain stable when the HashMap is extended.

The main use case is as part of a larger data structure where it can store structs computed from on other fields.
Individual entries may then need to be cleared when some fields are modified to ensure consistency.

The use of a cache allows to avoid repeating computations and leaking some implementation details (returning
owned objects or Rc can be a side effect of some internal design choices).


The current version is a proof of concept, with some potential room for improvement:
* The code is short and seems valid in single-threaded context, but uses unsafe code.
* It is not thread safe. A thread-safe implmentation of the same trait (using a Mutex) is possible.
* It uses an internal HashMap, but could be extended to other Indexable backend.

