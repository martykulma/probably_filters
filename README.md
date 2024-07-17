# Probabilistic data structures for fun and profit

...actually just for fun!

## Examining Counting Bloom Filters in Rust

A bloom filter is a data structure used to determine whether a entity is part of a set. It trades accuracy and functionality for space savings. A new key is hashed multiple times, and the hash values are used to set corresponding bits in a bit string/array. This structure often gets used to avoid costly lookups (generally I/O) that might yield no results.

A standard bloom filter doesn't allow for delete. A modification to a bloom filter is to use a counter instead of a bit, allowing adds to increment the counter, and deletes to decrement. This change also allows for different functionality.  Where a bloom filter only indicates a binary answer (part of set or not), a counting bloom filter can be used to determine how often a key has been added, letting us know how often it is accessed.


