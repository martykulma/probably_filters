use criterion::{criterion_group, criterion_main, Criterion};
use fasthash::metro;
use probably_filters::CuckooFilter;

pub fn bench_add(c: &mut Criterion) {
    let mut cbf = CuckooFilter::<metro::Hasher64_1>::new(500_000);
    c.bench_function("bench_add_entries", |b| {
        b.iter(|| {
            std::hint::black_box(for i in 0..1_000_000_u64 {
                cbf.add(i.to_ne_bytes().as_ref());
            });
        })
    });
}

pub fn bench_contains(c: &mut Criterion) {
    let mut cbf = CuckooFilter::<metro::Hasher64_1>::new(500_000);
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

criterion_group!(bench_cuckoo, bench_add, bench_contains,);
criterion_main!(bench_cuckoo);
