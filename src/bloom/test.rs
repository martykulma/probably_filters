use fasthash::{metro, murmur3};

use super::*;

macro_rules! test_add {
    ($($name:ident: $bits:expr,)*) => {
        $(
            #[test]
            fn $name () {
                let mut cbf = CountingBloomFilter::<murmur3::Hasher32>::with_bits_per_counter(9, 3, $bits).unwrap();
                let s1 = "Hello, world!".as_bytes();
                let s2 = "hello, world!".as_bytes();
                assert!(cbf.add(s1));
                assert!(cbf.contains(s1));
                assert!(!cbf.contains(s2));
            }
        )*

    };
}

test_add! {
    test_add_64: 64,
    test_add_32: 32,
    test_add_16: 16,
    test_add_8: 8,
    test_add_4: 4,
    test_add_2: 2,
    test_add_1: 1,
}

macro_rules! test_rm {
    ($($name:ident: $bits:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let mut cbf = CountingBloomFilter::<metro::Hasher64_1>::with_bits_per_counter(9, 3, $bits).unwrap();
                let s1 = "armadillo".as_bytes();
                let s2 = "armadill0".as_bytes();
                assert!(cbf.add(s1));
                assert!(cbf.add(s2));
                assert!(cbf.contains(s1));
                assert!(cbf.contains(s2));
                assert!(cbf.remove(s1));
                assert!(!cbf.contains(s1));
                assert!(cbf.contains(s2));
                assert!(cbf.remove(s2));
                assert!(!cbf.contains(s1));
                assert!(!cbf.contains(s2));
                assert_eq!(0_usize, cbf.counter_bins.iter().sum());
            }
        )*
    };
}

test_rm! {
    test_rm_64: 64,
    test_rm_32: 32,
    test_rm_16: 16,
    test_rm_8: 8,
    test_rm_4: 4,
    test_rm_2: 2,
    test_rm_1: 1,
}
// removal from empty filter doesn't cause counters to wrap
#[test]
fn test_remove_from_empty() {
    let mut cbf = CountingBloomFilter::<metro::Hasher64_1>::new(9, 3).unwrap();
    assert_eq!(0_usize, cbf.counter_bins.iter().sum());
    let s = "mystring".as_bytes();
    assert!(!cbf.remove(s));
    assert_eq!(0_usize, cbf.counter_bins.iter().sum());
}

// adding to filter that has been saturated doesn't cause counter to wrap
#[test]
fn test_add_to_full() {
    let mut cbf = CountingBloomFilter::<metro::Hasher64_1>::new(1024, 3).unwrap();
    // NOTE: this happens to hash to 3 distinct buckets, but may not for other
    // combinations of false positive probability, input, and hash function
    let s = "mystring".as_bytes();
    let loops = 260;
    let mut successful_adds = 0;
    for _ in 0..loops {
        if cbf.add(s) {
            successful_adds += 1;
        }
    }
    assert_eq!(2_i32.pow(4) - 1, successful_adds);
    assert_eq!(3, cbf.counter_bins.iter().filter(|&&v| v > 0).count());
}

// Verify remove() only decrements counters if the entry could have been
// added to the filter.
// initial state: [0,0,0,0]
// add s1:        [1,0,1,1]
// remove s2:     [1,0,1,1]
// remove s3:     [1,0,1,1]
// ... etc.
#[test]
fn test_rm_only_if_exists() {
    let mut cbf = CountingBloomFilter::<metro::Hasher64_1>::new(9, 3).unwrap();
    assert_eq!(0_usize, cbf.counter_bins.iter().sum());
    let s = "mystring".as_bytes();
    assert!(cbf.add(s));
    let expected: usize = cbf.counter_bins.iter().sum();

    // this passes so long as extra values do not collide with the initial
    for extra in ["redfish", "bluefish", "onefish", "twofish"] {
        assert!(!cbf.remove(extra.as_bytes()));
        assert_eq!(expected, cbf.counter_bins.iter().sum());
    }
}

#[test]
fn test_estimate() {
    let mut cbf = CountingBloomFilter::<metro::Hasher64_1>::new(9, 3).unwrap();
    let s = "wow".as_bytes();
    assert_eq!(0, cbf.estimate(s));
    for i in 1..6 {
        assert!(cbf.add(s));
        assert_eq!(i, cbf.estimate(s));
    }
    for i in (1..6).rev() {
        assert!(cbf.remove(s));
        assert_eq!(i - 1, cbf.estimate(s));
    }
    assert_eq!(0, cbf.estimate(s));
}

#[test]
fn test_invalid_hash_count() {
    macro_rules! test_invalid_hash_param {
        ($b:expr, $h:expr) => {
            let cbf = CountingBloomFilter::<metro::Hasher64_1>::new($b, $h);
            assert!(
                matches!(cbf, Err(Error::InvalidHashCount(_))),
                "Expected error for bins = {} hashes = {}",
                $b,
                $h
            );
        };
    }
    test_invalid_hash_param!(1, 2);
    test_invalid_hash_param!(1, 0);
}

#[test]
fn test_invalid_bin_count() {
    let cbf = CountingBloomFilter::<metro::Hasher64_1>::new(0, 0);
    assert!(matches!(cbf, Err(Error::InvalidBinCount(_))));
}

#[test]
fn test_max_counter() {
    let mut input = usize::BITS;
    let mut expected = !0_usize;
    let mut shift_bits = 0;
    while input > 0 {
        assert_eq!(expected, calc_max_counter(&input));
        input /= 2;
        shift_bits += input;
        expected = (expected << shift_bits) >> shift_bits;
    }
}
