use probably_filters::CountingBloomFilter;
use criterion::{criterion_group, criterion_main, Criterion};
use fasthash::metro;

pub fn bench_add_entries(c: &mut Criterion) {
    let mut cbf = CountingBloomFilter::<metro::Hasher64_1>::new(250_000, 4).unwrap();
    c.bench_function("bench_add_entries", |b| {
        b.iter(|| {
            std::hint::black_box(for i in 0..1_000_000_u64 {
                cbf.add(&i.to_ne_bytes()[..]);
            });
        })
    });
}

pub fn bench_contains(c: &mut Criterion) {
    let mut cbf = CountingBloomFilter::<metro::Hasher64_1>::new(250_000, 4).unwrap();
    for i in 0..1_000_000_u64 {
        cbf.add(&i.to_ne_bytes()[..]);
    }
    c.bench_function("bench_contains_existing", |b| {
        b.iter(|| {
            std::hint::black_box(for i in 0..1_000_000_u64 {
                cbf.contains(&i.to_ne_bytes()[..]);
            })
        })
    });
    c.bench_function("bench_contains_nonexisting", |b| {
        b.iter(|| {
            std::hint::black_box(for i in 1_000_000_u64..2_000_000_u64 {
                cbf.contains(&i.to_ne_bytes()[..]);
            })
        })
    });
}

criterion_group!(benches, bench_add_entries, bench_contains);
criterion_main!(benches);
