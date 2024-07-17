use core::num;
use fasthash::FastHasher;
use std::{collections::HashMap, marker::PhantomData};
use thiserror::Error;

#[cfg(test)]
mod test;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid hash count {0}: must be 0 < hash_count <= bin_count")]
    InvalidHashCount(u32),

    #[error("Invalid bin count {0}: must be > 0")]
    InvalidBinCount(usize),

    #[error("Invalid bits per counter {0}: must be < usize::BITS ({1})")]
    BitsPerCounterTooLarge(u32, u32),

    #[error("Invalid bits per counter {0}: must divide evenly into usize::BITS ({1})")]
    BitsPerCounterUnaligned(u32, u32),
}

const DEFAULT_BITS_PER_COUNTER: u32 = 4;

/// Implementation of a [counting bloom filter](https://en.wikipedia.org/wiki/Counting_Bloom_filter).
pub struct CountingBloomFilter<T>
where
    T: FastHasher<Seed = u32>,
{
    counter_bins: Vec<usize>,
    counter_max: usize,
    counters_per_bin: u32,
    bits_per_counter: u32,
    n_hashes: u32,
    _hasher: PhantomData<T>,
}

impl<T> CountingBloomFilter<T>
where
    T: FastHasher<Seed = u32>,
{
    /// Create a new counting bloom filter with 4 bits per counter.
    pub fn new(num_counters: usize, num_hashes: u32) -> Result<Self, Error> {
        Self::with_bits_per_counter(num_counters, num_hashes, DEFAULT_BITS_PER_COUNTER)
    }

    /// Create a new counting bloom filter with specified bits per counter.
    ///
    /// `bits_per_counter` must
    /// * be less than or equal to [usize::BITS]
    /// * divide evenly into [usize::BITS] ([usize::BITS] % `bits_per_counter` == 0)
    ///
    /// `num_counters` is a lower bound, number of bins actually allocated will be
    /// ceil(`num_counters` * `bits_per_counter` / [usize::BITS])
    ///
    /// `num_hashes` must be greater than 0 and less than, or equal to, `num_counters`
    pub fn with_bits_per_counter(
        num_counters: usize,
        num_hashes: u32,
        bits_per_counter: u32,
    ) -> Result<Self, Error> {
        if bits_per_counter > usize::BITS {
            return Err(Error::BitsPerCounterTooLarge(bits_per_counter, usize::BITS));
        }
        if usize::BITS % bits_per_counter != 0 {
            return Err(Error::BitsPerCounterUnaligned(
                bits_per_counter,
                usize::BITS,
            ));
        }
        if num_counters == 0 {
            return Err(Error::InvalidBinCount(num_counters));
        }
        if num_hashes == 0 || num_hashes as usize > num_counters {
            return Err(Error::InvalidHashCount(num_hashes));
        }

        let counters_per_bin = usize::BITS / bits_per_counter;
        let num_bins = num_counters.div_ceil(counters_per_bin as usize);
        Ok(CountingBloomFilter {
            counter_bins: vec![0; num_bins],
            counter_max: calc_max_counter(&bits_per_counter),
            counters_per_bin,
            bits_per_counter: bits_per_counter,
            n_hashes: num_hashes,
            _hasher: PhantomData,
        })
    }

    fn offsets(&self, hash: &usize) -> (usize, usize, usize) {
        // layout of counters
        // --------------- bin 0 ----------------- | --------------- bin 1 -----------------
        // 7    6    5    4    3    2    1    0    | 7    6    5    4    3    2    1    0
        // 1111 1111 1111 1111 1111 1111 1111 1111 | 1111 1111 1111 1111 1111 1111 1111 1111
        //
        // example for hash of 11 and default 4 bit counters
        // bin = hash (11) / counters_per_bin(8) = 1
        // shift = hash (11) % counters_per_bin(8) = 3 * bits_per_coutner(4) = 12
        // counter_mask = counter_max_val (15) << shift (12) = 0 1111 0000 0000 0000
        let bin = hash % self.counter_bins.len();
        // TODO: we know we are dealing with powers of 2 here, check if faster with bitwise ops
        let bitshift = (hash % self.counters_per_bin as usize) * self.bits_per_counter as usize;
        let counter_mask = self.counter_max << bitshift;
        (bin, bitshift, counter_mask)
    }

    /// Add an entry to the filter.  An entry can be added repeatedly, and each time
    /// counters in the associated bins are incremented.  This uses a saturating add, so
    /// once coutners have reached their max, they will no longer increase.
    ///
    /// This returns true if the entry was added or false if the counter was saturated (hence not added).
    pub fn add<'a, I>(&mut self, entry: I) -> bool
    where
        I: Into<&'a [u8]>,
    {
        let bytes: &[u8] = entry.into();
        let mut updates = HashMap::<usize, usize>::new();
        for mut h in (0..self.n_hashes).map(|seed| T::with_seed(seed.into())) {
            h.write(bytes);
            let hash = h.finish();
            let (bin, bitshift, counter_mask) = self.offsets(&(hash as usize));
            let mut counter = updates.get_mut(&bin).map_or_else(
                || (counter_mask & self.counter_bins[bin]) >> bitshift,
                |v| (counter_mask & *v) >> bitshift,
            );

            // if saturated, skip update
            if counter == self.counter_max {
                return false;
            }
            counter += 1;
            updates
                .entry(bin)
                .and_modify(|v| *v = (*v & !counter_mask) | (counter << bitshift))
                .or_insert((self.counter_bins[bin] & !counter_mask) | (counter << bitshift));
        }

        // update with new values
        for (bin, new_val) in updates {
            self.counter_bins[bin] = new_val;
        }
        true
    }

    /// Remove an entry from the filter.
    ///
    /// Decrements counters for bins associated with the entry iff the entry possibly existed.
    /// A lookup is performed before decrementing the values in the bins to ensure that all bins
    /// have a value > 0. This does not guarantee that the entry ever existed in the filter as
    /// these checks are subject to the false positive probability.
    /// This method uses a saturating subtraction, so counters do not wrap.
    ///
    /// This method returns false if the entry was not found (hence not removed), or true if it was.
    pub fn remove<'a, I>(&mut self, entry: I) -> bool
    where
        I: Into<&'a [u8]>,
    {
        let bytes: &[u8] = entry.into();
        let mut updates = HashMap::<usize, usize>::new();
        for seed in 0..self.n_hashes {
            let mut h = T::with_seed(seed);
            h.write(bytes);
            let hash = h.finish();
            let (bin, bitshift, counter_mask) = self.offsets(&(hash as usize));
            let mut counter = updates.get_mut(&bin).map_or_else(
                || (counter_mask & self.counter_bins[bin]) >> bitshift,
                |v| (counter_mask & *v) >> bitshift,
            );

            if counter == 0 {
                // one of the counters is 0, which means this key doesn't exist
                return false;
            }

            counter -= 1;
            updates
                .entry(bin)
                .and_modify(|v| *v = dbg!((*v & !counter_mask) | (counter << bitshift)))
                .or_insert(dbg!(
                    (self.counter_bins[bin] & !counter_mask) | (counter << bitshift)
                ));
        }

        // update with new values
        for (bin, new_val) in updates {
            self.counter_bins[bin] = new_val;
        }
        true
    }

    /// Determine if filter contains the provided entry.
    pub fn contains<'a, I>(&self, entry: I) -> bool
    where
        I: Into<&'a [u8]>,
    {
        let bytes: &[u8] = entry.into();
        (0..self.n_hashes)
            .map(|seed| {
                let mut h = T::with_seed(seed.into());
                h.write(bytes);
                let hash = h.finish();
                let (bin, bitshift, counter_mask) = self.offsets(&(hash as usize));
                (counter_mask & self.counter_bins[bin]) >> bitshift
            })
            .all(|v| v > 0)
    }

    /// Returns an estimate of the number of time entry exists in the filter.
    ///
    /// The estimate is determined as the minimum of counters for bins associated with this key.
    /// Counters support a maximum value of 255. If a key is added more than 255 times, it will
    /// increase the error rate of this filter and the estimate.  This estimate is also subject
    /// to the false positive probability
    pub fn estimate<'a, I>(&self, entry: I) -> usize
    where
        I: Into<&'a [u8]>,
    {
        let bytes: &[u8] = entry.into();
        (0..self.n_hashes)
            .map(|seed| {
                let mut h = T::with_seed(seed.into());
                h.write(bytes);
                let hash = h.finish();
                let (bin, bitshift, counter_mask) = self.offsets(&(hash as usize));
                (counter_mask & self.counter_bins[bin]) >> bitshift
            })
            .min()
            .unwrap_or_default()
    }
}

fn calc_max_counter(n_bits: &u32) -> usize {
    match n_bits {
        &usize::BITS => !0_usize,
        _ => 2_usize.pow(*n_bits) - 1,
    }
}
