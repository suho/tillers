use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tillers::services::WindowManager;

fn benchmark_window_positioning(c: &mut Criterion) {
    c.bench_function("window_positioning", |b| {
        b.iter(|| {
            // TODO: Implement when WindowManager is available
            black_box(());
        })
    });
}

criterion_group!(benches, benchmark_window_positioning);
criterion_main!(benches);
