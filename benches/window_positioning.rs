//! Performance benchmarks for window positioning operations
//!
//! Benchmarks the critical path of window positioning and tiling
//! to ensure sub-50ms response times for smooth user experience.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tillers::{
    macos::accessibility::{Point, Rect, Size},
    models::tiling_pattern::{LayoutAlgorithm, ResizeBehavior, TilingPattern},
    services::{
        tiling_engine::{TilingEngine, WindowLayout},
        window_manager::{WindowManager, WindowManagerConfig},
    },
};
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Create a benchmark runtime for async operations
fn create_runtime() -> Runtime {
    Runtime::new().expect("Failed to create Tokio runtime")
}

/// Create a window manager with test configuration
fn create_window_manager() -> Result<WindowManager, Box<dyn std::error::Error>> {
    let config = WindowManagerConfig {
        animation_enabled: false,
        animation_duration_ms: 0,
        focus_follows_mouse: false,
        auto_hide_cursor: false,
        respect_window_hints: true,
        performance_mode: true,
    };
    WindowManager::new(config)
}

/// Create a tiling engine for benchmarking
fn create_tiling_engine() -> TilingEngine {
    TilingEngine::new()
}

/// Create a test tiling pattern
fn create_test_pattern(algorithm: LayoutAlgorithm) -> TilingPattern {
    TilingPattern {
        id: Uuid::new_v4(),
        name: format!("{:?} Pattern", algorithm),
        layout_algorithm: algorithm,
        main_area_ratio: 0.6,
        gap_size: 10,
        window_margin: 20,
        max_windows: 10,
        resize_behavior: ResizeBehavior::Shrink,
    }
}

/// Create a test screen area (1920x1080)
fn create_test_area() -> Rect {
    Rect {
        origin: Point { x: 0.0, y: 0.0 },
        size: Size {
            width: 1920.0,
            height: 1080.0,
        },
    }
}

/// Create a smaller test area for stress testing
fn create_small_area() -> Rect {
    Rect {
        origin: Point { x: 100.0, y: 100.0 },
        size: Size {
            width: 800.0,
            height: 600.0,
        },
    }
}

/// Create window IDs for testing
fn create_window_ids(count: usize) -> Vec<u32> {
    (1..=count as u32).collect()
}

/// Benchmark tiling engine layout calculation for master-stack pattern
fn bench_master_stack_layout(c: &mut Criterion) {
    let rt = create_runtime();

    let window_counts = vec![1, 2, 5, 10, 20];

    for count in window_counts {
        c.bench_with_input(
            BenchmarkId::new("master_stack_layout", count),
            &count,
            |b, &window_count| {
                b.to_async(&rt).iter(|| async {
                    let engine = create_tiling_engine();
                    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
                    let area = create_test_area();
                    let window_ids = create_window_ids(window_count);

                    black_box(engine.layout_windows(&window_ids, &pattern, area).await)
                });
            },
        );
    }
}

/// Benchmark tiling engine layout calculation for grid pattern
fn bench_grid_layout(c: &mut Criterion) {
    let rt = create_runtime();

    let window_counts = vec![1, 4, 9, 16, 25];

    for count in window_counts {
        c.bench_with_input(
            BenchmarkId::new("grid_layout", count),
            &count,
            |b, &window_count| {
                b.to_async(&rt).iter(|| async {
                    let engine = create_tiling_engine();
                    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
                    let area = create_test_area();
                    let window_ids = create_window_ids(window_count);

                    black_box(engine.layout_windows(&window_ids, &pattern, area).await)
                });
            },
        );
    }
}

/// Benchmark tiling engine layout calculation for columns pattern
fn bench_columns_layout(c: &mut Criterion) {
    let rt = create_runtime();

    let window_counts = vec![1, 3, 5, 10, 15];

    for count in window_counts {
        c.bench_with_input(
            BenchmarkId::new("columns_layout", count),
            &count,
            |b, &window_count| {
                b.to_async(&rt).iter(|| async {
                    let engine = create_tiling_engine();
                    let pattern = create_test_pattern(LayoutAlgorithm::Columns);
                    let area = create_test_area();
                    let window_ids = create_window_ids(window_count);

                    black_box(engine.layout_windows(&window_ids, &pattern, area).await)
                });
            },
        );
    }
}

/// Benchmark window positioning with different area sizes
fn bench_layout_different_areas(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("layout_small_area", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = create_tiling_engine();
            let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
            let area = create_small_area();
            let window_ids = create_window_ids(5);

            black_box(engine.layout_windows(&window_ids, &pattern, area).await)
        });
    });

    c.bench_function("layout_large_area", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = create_tiling_engine();
            let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
            let area = Rect {
                origin: Point { x: 0.0, y: 0.0 },
                size: Size {
                    width: 3840.0,
                    height: 2160.0,
                }, // 4K resolution
            };
            let window_ids = create_window_ids(5);

            black_box(engine.layout_windows(&window_ids, &pattern, area).await)
        });
    });
}

/// Benchmark rapid layout recalculation (simulating window changes)
fn bench_rapid_layout_recalculation(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("rapid_layout_recalculation", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = create_tiling_engine();
            let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
            let area = create_test_area();

            // Simulate rapid changes: add/remove windows
            let sequences = vec![
                create_window_ids(1),
                create_window_ids(2),
                create_window_ids(3),
                create_window_ids(2),
                create_window_ids(4),
                create_window_ids(1),
            ];

            for window_ids in sequences {
                let _ = black_box(engine.layout_windows(&window_ids, &pattern, area).await);
            }
        });
    });
}

/// Benchmark different gap sizes and margins
fn bench_layout_with_spacing_variations(c: &mut Criterion) {
    let rt = create_runtime();

    let gap_sizes = vec![0, 5, 10, 20, 50];

    for gap_size in gap_sizes {
        c.bench_with_input(
            BenchmarkId::new("layout_gap_size", gap_size),
            &gap_size,
            |b, &gap| {
                b.to_async(&rt).iter(|| async {
                    let engine = create_tiling_engine();
                    let mut pattern = create_test_pattern(LayoutAlgorithm::Grid);
                    pattern.gap_size = gap;
                    pattern.window_margin = gap / 2;

                    let area = create_test_area();
                    let window_ids = create_window_ids(9); // 3x3 grid

                    black_box(engine.layout_windows(&window_ids, &pattern, area).await)
                });
            },
        );
    }
}

/// Benchmark window positioning with extreme window counts
fn bench_extreme_window_counts(c: &mut Criterion) {
    let rt = create_runtime();

    let extreme_counts = vec![50, 100, 200];

    for count in extreme_counts {
        c.bench_with_input(
            BenchmarkId::new("extreme_window_count", count),
            &count,
            |b, &window_count| {
                b.to_async(&rt).iter(|| async {
                    let engine = create_tiling_engine();
                    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
                    let area = create_test_area();
                    let window_ids = create_window_ids(window_count);

                    black_box(engine.layout_windows(&window_ids, &pattern, area).await)
                });
            },
        );
    }
}

/// Benchmark main area ratio variations
fn bench_main_area_ratio_variations(c: &mut Criterion) {
    let rt = create_runtime();

    let ratios = vec![0.3, 0.5, 0.6, 0.7, 0.9];

    for ratio in ratios {
        c.bench_with_input(
            BenchmarkId::new("main_area_ratio", (ratio * 10.0) as u32),
            &ratio,
            |b, &ratio_val| {
                b.to_async(&rt).iter(|| async {
                    let engine = create_tiling_engine();
                    let mut pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
                    pattern.main_area_ratio = ratio_val;

                    let area = create_test_area();
                    let window_ids = create_window_ids(6);

                    black_box(engine.layout_windows(&window_ids, &pattern, area).await)
                });
            },
        );
    }
}

/// Benchmark concurrent layout calculations
fn bench_concurrent_layout_calculations(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("concurrent_layout_calculations", |b| {
        b.to_async(&rt).iter(|| async {
            use tokio::task::JoinSet;

            let engine = create_tiling_engine();
            let mut join_set = JoinSet::new();

            // Spawn multiple concurrent layout calculations
            for i in 0..5 {
                let engine_ref = &engine;
                join_set.spawn(async move {
                    let pattern = create_test_pattern(match i % 3 {
                        0 => LayoutAlgorithm::MasterStack,
                        1 => LayoutAlgorithm::Grid,
                        _ => LayoutAlgorithm::Columns,
                    });
                    let area = create_test_area();
                    let window_ids = create_window_ids(5 + i);

                    engine_ref.layout_windows(&window_ids, &pattern, area).await
                });
            }

            // Wait for all calculations to complete
            while let Some(result) = join_set.join_next().await {
                let _ = black_box(result);
            }
        });
    });
}

/// Benchmark layout calculation with window positioning
fn bench_window_positioning_simulation(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("window_positioning_simulation", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = create_tiling_engine();
            let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
            let area = create_test_area();
            let window_ids = create_window_ids(5);

            // Calculate layout
            let layouts = engine
                .layout_windows(&window_ids, &pattern, area)
                .await
                .unwrap();

            // Simulate window positioning operations
            for layout in layouts {
                // Simulate setting window frame (would normally call macOS APIs)
                let _ = black_box((layout.window_id, layout.frame));

                // Simulate validation
                let frame = &layout.frame;
                let _ = black_box(frame.origin.x >= 0.0 && frame.origin.y >= 0.0);
                let _ = black_box(frame.size.width > 0.0 && frame.size.height > 0.0);
            }
        });
    });
}

/// Benchmark metrics collection overhead
fn bench_metrics_collection_overhead(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("metrics_collection", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = create_tiling_engine();
            let pattern = create_test_pattern(LayoutAlgorithm::Grid);
            let area = create_test_area();
            let window_ids = create_window_ids(4);

            // Perform layout with metrics collection
            let _ = black_box(engine.layout_windows(&window_ids, &pattern, area).await);

            // Get metrics (overhead of metrics collection)
            let metrics = black_box(engine.metrics().await);
            let _ = black_box(metrics.layout_requests);
            let _ = black_box(metrics.last_window_count);
            let _ = black_box(metrics.last_algorithm);
        });
    });
}

/// Benchmark error handling paths
fn bench_error_handling_paths(c: &mut Criterion) {
    let rt = create_runtime();

    c.bench_function("error_handling", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = create_tiling_engine();
            let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
            let area = create_test_area();

            // Test error path: empty window list
            let empty_windows = vec![];
            let result = engine.layout_windows(&empty_windows, &pattern, area).await;
            let _ = black_box(result.is_err());

            // Test edge case: single window
            let single_window = vec![1];
            let result = engine.layout_windows(&single_window, &pattern, area).await;
            let _ = black_box(result);

            // Test edge case: very small area
            let tiny_area = Rect {
                origin: Point { x: 0.0, y: 0.0 },
                size: Size {
                    width: 10.0,
                    height: 10.0,
                },
            };
            let windows = vec![1, 2];
            let result = engine.layout_windows(&windows, &pattern, tiny_area).await;
            let _ = black_box(result);
        });
    });
}

// Configure benchmark groups
criterion_group!(
    benches,
    bench_master_stack_layout,
    bench_grid_layout,
    bench_columns_layout,
    bench_layout_different_areas,
    bench_rapid_layout_recalculation,
    bench_layout_with_spacing_variations,
    bench_extreme_window_counts,
    bench_main_area_ratio_variations,
    bench_concurrent_layout_calculations,
    bench_window_positioning_simulation,
    bench_metrics_collection_overhead,
    bench_error_handling_paths
);

criterion_main!(benches);
