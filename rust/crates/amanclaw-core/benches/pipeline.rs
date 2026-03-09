use criterion::{Criterion, criterion_group, criterion_main};

use amanclaw_core::diagnostics::run_startup_diagnostics;
use amanclaw_traits::config::AppConfig;

fn bench_diagnostics(c: &mut Criterion) {
    let config = AppConfig::default();

    c.bench_function("run_startup_diagnostics", |b| {
        b.iter(|| run_startup_diagnostics(&config))
    });
}

criterion_group!(benches, bench_diagnostics);
criterion_main!(benches);
