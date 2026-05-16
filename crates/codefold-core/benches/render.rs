use std::path::PathBuf;

use codefold_core::{read, Level};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn bench_python(c: &mut Criterion) {
    let paths = [
        ("auth", fixture("python/auth.py")),
        ("heavy", fixture("python/heavy.py")),
    ];

    for (label, path) in &paths {
        let mut group = c.benchmark_group(format!("python/{label}"));
        for level in [Level::Full, Level::Signatures, Level::Bodies] {
            group.bench_function(format!("{:?}", level), |b| {
                b.iter(|| {
                    let r = read(black_box(path), level).unwrap();
                    black_box(r);
                });
            });
        }
        group.finish();
    }
}

fn bench_typescript(c: &mut Criterion) {
    let path = fixture("typescript/auth.ts");
    let mut group = c.benchmark_group("typescript/auth");
    for level in [Level::Full, Level::Signatures, Level::Bodies] {
        group.bench_function(format!("{:?}", level), |b| {
            b.iter(|| {
                let r = read(black_box(&path), level).unwrap();
                black_box(r);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_python, bench_typescript);
criterion_main!(benches);
