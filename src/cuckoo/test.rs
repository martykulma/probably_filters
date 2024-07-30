use fasthash::murmur3;

use super::CuckooFilter;

#[test]
fn test_add() {
    let mut cf = CuckooFilter::<murmur3::Hasher32>::new(5);
    let v = "value";
    cf.add(v.as_bytes());
    assert_eq!(1, cf.bins.iter().map(|v| v.len()).sum::<usize>());
    assert!(cf.contains(v.as_bytes()));
}

#[test]
fn test_add_duplicate() {
    let mut cf = CuckooFilter::<murmur3::Hasher32>::new(13);
    let v = "value";
    cf.add(v.as_bytes());
    assert!(cf.contains(v.as_bytes()));
    cf.add(v.as_bytes());
    assert!(cf.contains(v.as_bytes()));
    let fingerprints = cf
        .bins
        .iter()
        .flat_map(|arr| arr.iter())
        .collect::<Vec<_>>();
    assert_eq!(2, fingerprints.len());
    assert_eq!(fingerprints[0], fingerprints[1]);
}

#[test]
fn test_remove() {
    let mut cf = CuckooFilter::<murmur3::Hasher32>::new(8);
    let v = "value";
    cf.add(v.as_bytes());
    assert!(cf.contains(v.as_bytes()));
    cf.remove(v.as_bytes());
    assert!(!cf.contains(v.as_bytes()));
    assert!(!cf.remove(v.as_bytes()));
}

#[test]
fn test_fill() {
    let mut cf = CuckooFilter::<murmur3::Hasher32>::new(512);
    for i in 0..1024u64 {
        assert!(cf.add(&i.to_ne_bytes()), "{}", i);
    }
    assert_eq!(1024, cf.bins.iter().map(|v| v.len()).sum::<usize>());
    for i in 0..1024u64 {
        assert!(cf.remove(&i.to_ne_bytes()), "{}", i);
    }
    assert_eq!(0, cf.bins.iter().map(|v| v.len()).sum::<usize>());
}
