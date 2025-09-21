use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tillers::services::WorkspaceManager;

fn benchmark_workspace_switch(c: &mut Criterion) {
    c.bench_function("workspace_switch", |b| {
        b.iter(|| {
            // TODO: Implement when WorkspaceManager is available
            black_box(());
        })
    });
}

criterion_group!(benches, benchmark_workspace_switch);
criterion_main!(benches);
