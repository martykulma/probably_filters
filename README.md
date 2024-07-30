# Probabilistic data structures for fun and profit

...actually just for fun!

## Examining Counting Bloom Filters in Rust

A bloom filter is a data structure used to determine whether a entity is part of a set. It trades accuracy and functionality for space savings. A new key is hashed multiple times, and the hash values are used to set corresponding bits in a bit string/array. This structure often gets used to avoid costly lookups (generally I/O) that might yield no results.

A standard bloom filter doesn't allow for delete. A modification to a bloom filter is to use a counter instead of a bit, allowing adds to increment the counter, and deletes to decrement. This change also allows for different functionality.  Where a bloom filter only indicates a binary answer (part of set or not), a counting bloom filter can be used to determine how often a key has been added, letting us know how often it is accessed.


## What about this Cuckoo filter?

This is another probabilitistic data structure with a goal similar to the counting bloom filter. Test set membership, allow for deletes.  The implementation of a Cuckoo filter stores a fingerprint of the key at one of two locations, determined by the hashing scheme. If both locations are full, a random fingerprint from one of the 2 locations is replaced by the new fingerprint.  The removed fingerprint is placed into it's alternate location.  If there is no space available, a random fingerprint is removed and .... well you get the idea.  This is a bound to some maximum number of displacements. It does mean that as the filter occupancy iincreases, the insertion speed slows down.  Unlike bloom filters, Cuckoo only uses 3 hash operations (fingerprint, hash of entry, hash of fingerprint).