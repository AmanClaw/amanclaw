use criterion::{black_box, criterion_group, criterion_main, Criterion};

use amanclaw_prayer_times::{calculate, CalculationMethod};
use chrono::NaiveDate;

fn bench_prayer_time_calculation(c: &mut Criterion) {
    let date = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();

    // Kuala Lumpur: 3.1390° N, 101.6869° E, UTC+8
    c.bench_function("prayer_times_mwl_kl", |b| {
        b.iter(|| {
            calculate(
                black_box(date),
                black_box(3.1390),
                black_box(101.6869),
                black_box(8.0),
                black_box(CalculationMethod::MWL),
            )
        })
    });

    // New York: 40.7128° N, 74.0060° W, UTC-5
    c.bench_function("prayer_times_isna_nyc", |b| {
        b.iter(|| {
            calculate(
                black_box(date),
                black_box(40.7128),
                black_box(-74.0060),
                black_box(-5.0),
                black_box(CalculationMethod::ISNA),
            )
        })
    });

    let all_methods = [
        CalculationMethod::MWL,
        CalculationMethod::ISNA,
        CalculationMethod::Egyptian,
        CalculationMethod::Karachi,
        CalculationMethod::UmmAlQura,
        CalculationMethod::JAKIM,
    ];

    c.bench_function("prayer_times_all_methods", |b| {
        b.iter(|| {
            for method in &all_methods {
                calculate(
                    black_box(date),
                    black_box(3.1390),
                    black_box(101.6869),
                    black_box(8.0),
                    black_box(*method),
                );
            }
        })
    });
}

criterion_group!(benches, bench_prayer_time_calculation);
criterion_main!(benches);
