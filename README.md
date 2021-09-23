Caching map
==========

This crate provides a map (hidden behind an UnsafeCell) which accepts new entries while immutable.
References to existing entries can be obtained with the lifetime of the caching map.
To ensure that the returned references remain valid, existing entries can only be removed or replaced
when the map is mutable.


