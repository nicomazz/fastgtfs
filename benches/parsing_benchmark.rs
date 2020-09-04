use criterion::{criterion_group, criterion_main, Criterion};

use fastgtfs::raw_parser::RawParser;
use fastgtfs::test_utils::generate_serialized_data;

fn generate_serialized_data_benchmark(c: &mut Criterion) {
    c.bench_function("write serialized data", |b| {
        b.iter(|| {
            generate_serialized_data();
        })
    });
}

fn parsing_benchmark(c: &mut Criterion) {
    generate_serialized_data();
    c.bench_function("read serialized data", |b| {
        b.iter(|| {
            let gtfs = RawParser::read_preprocessed_data_from_default();
            println!("Parsed with {} trips", gtfs.trips.len());
        })
    });
}

criterion_group!(
    benches,
    parsing_benchmark,
    generate_serialized_data_benchmark
);
criterion_main!(benches);
