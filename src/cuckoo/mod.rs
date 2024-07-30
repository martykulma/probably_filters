use std::{
    hash::{DefaultHasher, Hasher},
    marker::PhantomData,
};

use fasthash::FastHasher;
use rand::{thread_rng, RngCore};

#[cfg(test)]
mod test;

/// Implements a cuckoo filter.  This is a probablisitic data structure akin to the bloom filter,
/// which is used for set membership queries with some amount of error.
///
/// A cuckoo filter stores the fingerprint for a key in an array. There are 2 possible candidate locations in the array, if the first
/// location is full, the other location is used.  If both are full, then the filter initiates a series of swaps, moving an existing
/// fingerprint to its alternate location. The number of swaps is bounded by the implementation. Each location can 1 or more entries.

pub struct CuckooFilter<T>
where
    T: FastHasher<Seed = u32>,
{
    // this probably doesn't need to be a Vec<Vec<u8>> -- will convert to Vec<usize> and limit the fingerprint
    // bit length
    bins: Vec<Vec<u8>>,
    max_kicks: u32, // how many times can we move fingerprints between bins
    _hasher: PhantomData<T>,
}

#[allow(dead_code)]
impl<T> CuckooFilter<T>
where
    T: FastHasher<Seed = u32>,
{
    pub fn new(num_bins: usize) -> Self {
        Self::with_all_the_levers(num_bins, 4, 100)
    }

    pub fn with_all_the_levers(num_bins: usize, entries_per_bin: usize, max_kicks: u32) -> Self {
        // Can't use vec![Vec::with_capacity(4); num_bins] as the macro uses
        // clone, and clone carries len forward, not capacity.
        CuckooFilter {
            bins: (0..num_bins)
                .map(|_| Vec::with_capacity(entries_per_bin))
                .collect::<Vec<_>>(),
            max_kicks,
            _hasher: PhantomData,
        }
    }

    pub fn add<I>(&mut self, entry: I) -> bool
    where
        I: AsRef<[u8]>,
    {
        let mut fingerprint = Self::fingerprint(entry.as_ref());
        let mut i = Self::hash(entry.as_ref()) as usize % self.bins.len();

        for attempt in 0..self.max_kicks {
            let bin = &mut self.bins[i];
            if bin.len() < bin.capacity() {
                bin.push(fingerprint);
                return true;
            }
            if attempt != 0 {
                let kick_idx = thread_rng().next_u32() as usize % bin.len();
                let kicked = bin.swap_remove(kick_idx);
                bin.push(fingerprint);
                fingerprint = kicked;
            }
            i = (i ^ Self::hash(&fingerprint.to_ne_bytes()) as usize) % self.bins.len();
        }
        false
    }

    pub fn remove<I>(&mut self, entry: I) -> bool
    where
        I: AsRef<[u8]>,
    {
        let fingerprint = Self::fingerprint(entry.as_ref());
        let i = Self::hash(entry.as_ref()) as usize % self.bins.len();

        let bin = &mut self.bins[i];
        if let Some(rmi) = bin.iter().position(|v| *v == fingerprint) {
            bin.swap_remove(rmi);
            return true;
        }
        let i = (i ^ Self::hash(&fingerprint.to_ne_bytes()) as usize) % self.bins.len();
        let bin = &mut self.bins[i];
        if let Some(rmi) = bin.iter().position(|v| *v == fingerprint) {
            bin.swap_remove(rmi);
            return true;
        }
        false
    }

    pub fn contains<I>(&self, entry: I) -> bool
    where
        I: AsRef<[u8]>,
    {
        let fingerprint = Self::fingerprint(entry.as_ref());
        let i = Self::hash(entry.as_ref()) as usize % self.bins.len();
        !self.bins[i].is_empty() && self.bins[i].contains(&fingerprint) || {
            let i = (i ^ Self::hash(&fingerprint.to_ne_bytes()) as usize) % self.bins.len();
            !self.bins[i].is_empty() && self.bins[i].contains(&fingerprint)
        }
    }

    // Technically doesn't need to be in the impl block, but hash is, so it feels odd to leave this out
    fn fingerprint(bytes: &[u8]) -> u8 {
        let mut hasher = DefaultHasher::new();
        hasher.write(bytes);
        hasher.finish() as u8
    }

    fn hash(bytes: &[u8]) -> u64 {
        let mut hash = T::new();
        hash.write(bytes);
        hash.finish()
    }
}
