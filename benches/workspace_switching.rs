//! Performance benchmarks for workspace switching operations
//!
//! Benchmarks the critical path of workspace switching to ensure
//! sub-200ms response times for optimal user experience.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tillers::{
    models::{
        workspace::{Workspace, WorkspaceKind},
        tiling_pattern::{TilingPattern, LayoutAlgorithm, ResizeBehavior},
    },
    services::{
        workspace_manager::{WorkspaceManager, WorkspaceManagerConfig},
        tiling_engine::TilingEngine,
    },
    WorkspaceCreateRequest,
};
use tokio::runtime::Runtime;
use uuid::Uuid;
use std::time::Duration;

/// Create a benchmark runtime for async operations
fn create_runtime() -> Runtime {
    Runtime::new().expect("Failed to create Tokio runtime")
}

/// Create a workspace manager with test configuration
fn create_workspace_manager() -> WorkspaceManager {
    let config = WorkspaceManagerConfig {
        max_workspaces: 50,
        default_tiling_pattern_id: Some(Uuid::new_v4()),
        auto_save_enabled: false,
        workspace_timeout: Duration::from_secs(300),
        performance_mode: true,
    };
    WorkspaceManager::new(config)
}

/// Create a test tiling pattern
fn create_test_pattern() -> TilingPattern {
    TilingPattern {
        id: Uuid::new_v4(),
        name: "Test Pattern".to_string(),
        layout_algorithm: LayoutAlgorithm::MasterStack,
        main_area_ratio: 0.6,
        gap_size: 10,
        window_margin: 20,
        max_windows: 10,
        resize_behavior: ResizeBehavior::Shrink,
    }
}

/// Create a workspace creation request
fn create_workspace_request(index: usize) -> WorkspaceCreateRequest {
    WorkspaceCreateRequest {
        name: format!("Benchmark Workspace {}", index),
        description: Some(format!("Benchmark workspace for performance testing {}", index)),
        keyboard_shortcut: format!("opt+{}", (index % 9) + 1),
        tiling_pattern_id: Some(Uuid::new_v4()),
        auto_arrange: Some(true),
    }
}

/// Benchmark workspace creation performance
fn bench_workspace_creation(c: &mut Criterion) {
    let rt = create_runtime();
    
    c.bench_function("workspace_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = create_workspace_manager();
            let pattern = create_test_pattern();
            let request = create_workspace_request(1);
            
            black_box(manager.create_workspace(request, pattern.id).await)
        });
    });
}

/// Benchmark workspace switching with varying numbers of existing workspaces
fn bench_workspace_switching(c: &mut Criterion) {
    let rt = create_runtime();
    
    // Test with different numbers of existing workspaces
    let workspace_counts = vec![1, 5, 10, 25, 50];
    
    for count in workspace_counts {
        c.bench_with_input(
            BenchmarkId::new("workspace_switching", count),
            &count,
            |b, &workspace_count| {
                b.to_async(&rt).iter(|| async {
                    let manager = create_workspace_manager();
                    let pattern = create_test_pattern();
                    
                    // Create workspaces
                    let mut workspace_ids = Vec::new();
                    for i in 0..workspace_count {
                        let request = create_workspace_request(i);
                        let id = manager.create_workspace(request, pattern.id).await.unwrap();
                        workspace_ids.push(id);
                    }
                    
                    // Benchmark switching to the last workspace
                    let target_id = workspace_ids.last().unwrap();
                    black_box(manager.switch_to_workspace(*target_id).await)
                });
            },
        );
    }
}

/// Benchmark workspace listing performance
fn bench_workspace_listing(c: &mut Criterion) {
    let rt = create_runtime();
    
    let workspace_counts = vec![10, 25, 50, 100];
    
    for count in workspace_counts {
        c.bench_with_input(
            BenchmarkId::new("workspace_listing", count),
            &count,
            |b, &workspace_count| {
                b.to_async(&rt).iter_with_setup(
                    // Setup: create workspaces
                    || {
                        rt.block_on(async {
                            let manager = create_workspace_manager();
                            let pattern = create_test_pattern();
                            
                            for i in 0..workspace_count {
                                let request = create_workspace_request(i);
                                let _ = manager.create_workspace(request, pattern.id).await.unwrap();
                            }
                            
                            manager
                        })
                    },
                    // Benchmark: list workspaces
                    |manager| async move {
                        black_box(manager.list_workspaces().await)
                    },
                );
            },
        );
    }
}

/// Benchmark workspace search by name
fn bench_workspace_search(c: &mut Criterion) {
    let rt = create_runtime();
    
    c.bench_function("workspace_search", |b| {
        b.to_async(&rt).iter_with_setup(
            // Setup: create 50 workspaces with varied names
            || {
                rt.block_on(async {
                    let manager = create_workspace_manager();
                    let pattern = create_test_pattern();
                    
                    let names = vec![
                        "Development", "Testing", "Production", "Staging", "Research",
                        "Documentation", "Design", "Analysis", "Review", "Deployment"
                    ];
                    
                    for i in 0..50 {
                        let name = format!("{} {}", names[i % names.len()], i / names.len());
                        let request = WorkspaceCreateRequest {
                            name: name.clone(),
                            description: Some(format!("Workspace for {}", name)),
                            keyboard_shortcut: format!("opt+{}", (i % 9) + 1),
                            tiling_pattern_id: Some(pattern.id),
                            auto_arrange: Some(true),
                        };
                        let _ = manager.create_workspace(request, pattern.id).await.unwrap();
                    }
                    
                    manager
                })
            },
            // Benchmark: search workspaces
            |manager| async move {
                black_box(manager.find_workspaces_by_name("Development").await)
            },
        );
    });
}

/// Benchmark workspace deletion
fn bench_workspace_deletion(c: &mut Criterion) {
    let rt = create_runtime();
    
    c.bench_function("workspace_deletion", |b| {
        b.to_async(&rt).iter_with_setup(
            // Setup: create a workspace to delete
            || {
                rt.block_on(async {
                    let manager = create_workspace_manager();
                    let pattern = create_test_pattern();
                    let request = create_workspace_request(1);
                    let id = manager.create_workspace(request, pattern.id).await.unwrap();
                    (manager, id)
                })
            },
            // Benchmark: delete workspace
            |(manager, workspace_id)| async move {
                black_box(manager.delete_workspace(workspace_id).await)
            },
        );
    });
}

/// Benchmark getting active workspace
fn bench_get_active_workspace(c: &mut Criterion) {
    let rt = create_runtime();
    
    c.bench_function("get_active_workspace", |b| {
        b.to_async(&rt).iter_with_setup(
            // Setup: create workspaces and set one as active
            || {
                rt.block_on(async {
                    let manager = create_workspace_manager();
                    let pattern = create_test_pattern();
                    
                    // Create multiple workspaces
                    let mut workspace_ids = Vec::new();
                    for i in 0..10 {
                        let request = create_workspace_request(i);
                        let id = manager.create_workspace(request, pattern.id).await.unwrap();
                        workspace_ids.push(id);
                    }
                    
                    // Switch to middle workspace
                    let _ = manager.switch_to_workspace(workspace_ids[5]).await;
                    
                    manager
                })
            },
            // Benchmark: get active workspace
            |manager| async move {
                black_box(manager.get_active_workspace().await)
            },
        );
    });
}

/// Benchmark workspace count retrieval
fn bench_workspace_count(c: &mut Criterion) {
    let rt = create_runtime();
    
    let workspace_counts = vec![0, 10, 50, 100];
    
    for count in workspace_counts {
        c.bench_with_input(
            BenchmarkId::new("workspace_count", count),
            &count,
            |b, &workspace_count| {
                b.to_async(&rt).iter_with_setup(
                    // Setup: create specified number of workspaces
                    || {
                        rt.block_on(async {
                            let manager = create_workspace_manager();
                            let pattern = create_test_pattern();
                            
                            for i in 0..workspace_count {
                                let request = create_workspace_request(i);
                                let _ = manager.create_workspace(request, pattern.id).await.unwrap();
                            }
                            
                            manager
                        })
                    },
                    // Benchmark: get workspace count
                    |manager| async move {
                        black_box(manager.get_workspace_count().await)
                    },
                );
            },
        );
    }
}

/// Benchmark rapid workspace switching (simulating user behavior)
fn bench_rapid_workspace_switching(c: &mut Criterion) {
    let rt = create_runtime();
    
    c.bench_function("rapid_workspace_switching", |b| {
        b.to_async(&rt).iter_with_setup(
            // Setup: create 5 workspaces
            || {
                rt.block_on(async {
                    let manager = create_workspace_manager();
                    let pattern = create_test_pattern();
                    
                    let mut workspace_ids = Vec::new();
                    for i in 0..5 {
                        let request = create_workspace_request(i);
                        let id = manager.create_workspace(request, pattern.id).await.unwrap();
                        workspace_ids.push(id);
                    }
                    
                    (manager, workspace_ids)
                })
            },
            // Benchmark: rapidly switch between workspaces
            |(manager, workspace_ids)| async move {
                // Simulate rapid switching pattern: 0 -> 2 -> 1 -> 4 -> 0
                let switch_pattern = vec![0, 2, 1, 4, 0];
                
                for &index in &switch_pattern {
                    let _ = black_box(manager.switch_to_workspace(workspace_ids[index]).await);
                }
            },
        );
    });
}

// Configure benchmark groups
criterion_group!(
    benches,
    bench_workspace_creation,
    bench_workspace_switching,
    bench_workspace_listing,
    bench_workspace_search,
    bench_workspace_deletion,
    bench_get_active_workspace,
    bench_workspace_count,
    bench_rapid_workspace_switching
);

criterion_main!(benches);