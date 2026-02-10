use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xconfig::kconfig::Parser;
use std::path::PathBuf;

fn bench_parse_simple(c: &mut Criterion) {
    let kconfig = PathBuf::from("examples/sample_project/Kconfig");
    let srctree = PathBuf::from("examples/sample_project");
    
    c.bench_function("parse_simple_kconfig", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(&kconfig), black_box(&srctree)).unwrap();
            parser.parse()
        })
    });
}

criterion_group!(benches, bench_parse_simple);
criterion_main!(benches);
