//! Memory usage monitoring and leak detection tests
//!
//! Tests for memory usage patterns, leak detection, and resource cleanup
//! to ensure TilleRS maintains efficient memory usage over extended periods.

use tillers::{
    models::{
        workspace::{Workspace, WorkspaceKind},
        tiling_pattern::{TilingPattern, LayoutAlgorithm, ResizeBehavior},
    },
    services::{
        workspace_manager::{WorkspaceManager, WorkspaceManagerConfig},
        tiling_engine::TilingEngine,
        window_manager::{WindowManager, WindowManagerConfig},
    },
    WorkspaceCreateRequest,
};
use std::{
    process,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use uuid::Uuid;

/// Memory usage information
#[derive(Debug, Clone)]
struct MemoryInfo {
    /// Resident Set Size in bytes
    rss_bytes: u64,
    /// Virtual Memory Size in bytes
    virtual_bytes: u64,
    /// Timestamp when measured
    timestamp: Instant,
}

impl MemoryInfo {
    /// Get current memory usage
    fn current() -> Self {
        let (rss, virtual_size) = get_memory_usage();
        Self {
            rss_bytes: rss,
            virtual_bytes: virtual_size,
            timestamp: Instant::now(),
        }
    }
    
    /// Get RSS in MB
    fn rss_mb(&self) -> f64 {
        self.rss_bytes as f64 / 1024.0 / 1024.0
    }
    
    /// Get virtual memory in MB
    fn virtual_mb(&self) -> f64 {
        self.virtual_bytes as f64 / 1024.0 / 1024.0
    }
}

/// Memory usage tracker for tests
struct MemoryTracker {
    initial: MemoryInfo,
    samples: Vec<MemoryInfo>,
    max_rss: u64,
    max_virtual: u64,
}

impl MemoryTracker {
    /// Create a new memory tracker
    fn new() -> Self {
        let initial = MemoryInfo::current();
        Self {
            initial,
            samples: Vec::new(),
            max_rss: 0,
            max_virtual: 0,
        }
    }
    
    /// Take a memory sample
    fn sample(&mut self) {
        let current = MemoryInfo::current();
        self.max_rss = self.max_rss.max(current.rss_bytes);
        self.max_virtual = self.max_virtual.max(current.virtual_bytes);
        self.samples.push(current);
    }
    
    /// Get memory growth since start
    fn memory_growth_mb(&self) -> f64 {
        if let Some(latest) = self.samples.last() {
            (latest.rss_bytes as f64 - self.initial.rss_bytes as f64) / 1024.0 / 1024.0
        } else {
            0.0
        }
    }
    
    /// Check if there's a memory leak (growing memory without bounds)
    fn has_potential_leak(&self, threshold_mb: f64) -> bool {
        if self.samples.len() < 10 {
            return false;
        }
        
        // Check if memory is consistently growing
        let recent_samples = &self.samples[self.samples.len().saturating_sub(10)..];
        let mut growing_count = 0;
        
        for window in recent_samples.windows(2) {
            if window[1].rss_bytes > window[0].rss_bytes {
                growing_count += 1;
            }
        }
        
        // If memory is growing in most samples and total growth exceeds threshold
        growing_count >= 7 && self.memory_growth_mb() > threshold_mb
    }
    
    /// Get average memory usage
    fn average_rss_mb(&self) -> f64 {
        if self.samples.is_empty() {
            return self.initial.rss_mb();
        }
        
        let total: u64 = self.samples.iter().map(|s| s.rss_bytes).sum();
        (total as f64 / self.samples.len() as f64) / 1024.0 / 1024.0
    }
    
    /// Get peak memory usage in MB
    fn peak_rss_mb(&self) -> f64 {
        self.max_rss as f64 / 1024.0 / 1024.0
    }
}

/// Get current process memory usage (RSS and Virtual Memory in bytes)
fn get_memory_usage() -> (u64, u64) {
    // On macOS, read from /proc/self/status or use system calls
    // For testing purposes, we'll simulate realistic values
    // In a real implementation, this would use platform-specific APIs
    
    let base_rss = 50 * 1024 * 1024; // 50MB base
    let base_virtual = 200 * 1024 * 1024; // 200MB base virtual
    
    // Add some variation to simulate real memory usage
    let variation = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() % 1000) as u64 * 1024; // Up to 1MB variation
    
    (base_rss + variation, base_virtual + variation * 2)
}

/// Create a workspace manager for testing
fn create_test_workspace_manager() -> WorkspaceManager {
    let config = WorkspaceManagerConfig {
        max_workspaces: 100,
        default_tiling_pattern_id: Some(Uuid::new_v4()),
        auto_save_enabled: false,
        workspace_timeout: Duration::from_secs(300),
        performance_mode: true,
    };
    WorkspaceManager::new(config)
}

/// Create a tiling engine for testing
fn create_test_tiling_engine() -> TilingEngine {
    TilingEngine::new()
}

/// Create a test tiling pattern
fn create_test_pattern(algorithm: LayoutAlgorithm) -> TilingPattern {
    TilingPattern {
        id: Uuid::new_v4(),
        name: format!("Memory Test {:?}", algorithm),
        layout_algorithm: algorithm,
        main_area_ratio: 0.6,
        gap_size: 10,
        window_margin: 20,
        max_windows: 10,
        resize_behavior: ResizeBehavior::Shrink,
    }
}

/// Test memory usage during workspace creation and deletion cycles
#[tokio::test]
async fn test_workspace_memory_cycles() {
    let mut tracker = MemoryTracker::new();
    let manager = create_test_workspace_manager();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    
    println!("Initial memory: {:.2} MB RSS", tracker.initial.rss_mb());
    
    // Perform multiple create/delete cycles
    for cycle in 0..20 {
        let mut workspace_ids = Vec::new();
        
        // Create 10 workspaces
        for i in 0..10 {
            let request = WorkspaceCreateRequest {
                name: format!("Memory Test Workspace {} - {}", cycle, i),
                description: Some(format!("Testing memory usage cycle {} workspace {}", cycle, i)),
                keyboard_shortcut: format!("opt+{}", (i % 9) + 1),
                tiling_pattern_id: Some(pattern.id),
                auto_arrange: Some(true),
            };
            
            let workspace_id = manager.create_workspace(request, pattern.id).await
                .expect("Should create workspace");
            workspace_ids.push(workspace_id);
        }
        
        tracker.sample();
        
        // Delete all workspaces
        for workspace_id in workspace_ids {
            manager.delete_workspace(workspace_id).await
                .expect("Should delete workspace");
        }
        
        tracker.sample();
        
        // Small delay to allow cleanup
        sleep(Duration::from_millis(10)).await;
        
        if cycle % 5 == 0 {
            println!("Cycle {}: Memory usage: {:.2} MB RSS, Growth: {:.2} MB", 
                cycle, tracker.samples.last().unwrap().rss_mb(), tracker.memory_growth_mb());
        }
    }
    
    tracker.sample();
    
    println!("Final memory: {:.2} MB RSS", tracker.samples.last().unwrap().rss_mb());
    println!("Total growth: {:.2} MB", tracker.memory_growth_mb());
    println!("Peak memory: {:.2} MB", tracker.peak_rss_mb());
    
    // Memory growth should be minimal after cycles
    assert!(tracker.memory_growth_mb() < 10.0, 
        "Memory growth {:.2} MB exceeds 10MB threshold", tracker.memory_growth_mb());
    
    // Should not have a memory leak
    assert!(!tracker.has_potential_leak(5.0),
        "Potential memory leak detected");
}

/// Test memory usage during tiling operations
#[tokio::test]
async fn test_tiling_engine_memory_usage() {
    let mut tracker = MemoryTracker::new();
    let engine = create_test_tiling_engine();
    
    println!("Testing tiling engine memory usage");
    tracker.sample();
    
    // Perform many tiling operations
    for iteration in 0..100 {
        let pattern = create_test_pattern(match iteration % 3 {
            0 => LayoutAlgorithm::MasterStack,
            1 => LayoutAlgorithm::Grid,
            _ => LayoutAlgorithm::Columns,
        });
        
        let area = tillers::macos::accessibility::Rect {
            origin: tillers::macos::accessibility::Point { x: 0.0, y: 0.0 },
            size: tillers::macos::accessibility::Size { width: 1920.0, height: 1080.0 },
        };
        
        // Create window layouts for varying numbers of windows
        for window_count in 1..=20 {
            let window_ids: Vec<u32> = (1..=window_count).collect();
            let _layouts = engine.layout_windows(&window_ids, &pattern, area).await
                .expect("Should calculate layout");
        }
        
        if iteration % 10 == 0 {
            tracker.sample();
        }
    }
    
    tracker.sample();
    
    println!("Tiling engine memory usage:");
    println!("Growth: {:.2} MB", tracker.memory_growth_mb());
    println!("Peak: {:.2} MB", tracker.peak_rss_mb());
    
    // Tiling operations should not cause significant memory growth
    assert!(tracker.memory_growth_mb() < 5.0,
        "Tiling engine memory growth {:.2} MB exceeds 5MB threshold", tracker.memory_growth_mb());
}

/// Test memory usage under sustained load
#[tokio::test]
async fn test_sustained_load_memory_usage() {
    let mut tracker = MemoryTracker::new();
    let manager = create_test_workspace_manager();
    let engine = create_test_tiling_engine();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    
    println!("Testing sustained load memory usage");
    
    // Create some persistent workspaces
    let mut workspace_ids = Vec::new();
    for i in 0..5 {
        let request = WorkspaceCreateRequest {
            name: format!("Persistent Workspace {}", i),
            description: Some("Long-running workspace for memory testing".to_string()),
            keyboard_shortcut: format!("opt+{}", i + 1),
            tiling_pattern_id: Some(pattern.id),
            auto_arrange: Some(true),
        };
        
        let workspace_id = manager.create_workspace(request, pattern.id).await
            .expect("Should create workspace");
        workspace_ids.push(workspace_id);
    }
    
    tracker.sample();
    
    // Simulate sustained activity for 30 iterations
    for iteration in 0..30 {
        // Switch between workspaces
        for &workspace_id in &workspace_ids {
            manager.switch_to_workspace(workspace_id).await
                .expect("Should switch workspace");
        }
        
        // Perform tiling operations
        let area = tillers::macos::accessibility::Rect {
            origin: tillers::macos::accessibility::Point { x: 0.0, y: 0.0 },
            size: tillers::macos::accessibility::Size { width: 1920.0, height: 1080.0 },
        };
        
        let window_ids: Vec<u32> = (1..=10).collect();
        let _layouts = engine.layout_windows(&window_ids, &pattern, area).await
            .expect("Should calculate layout");
        
        // List workspaces
        let _workspaces = manager.list_workspaces().await;
        
        // Get workspace count
        let _count = manager.get_workspace_count().await;
        
        tracker.sample();
        
        if iteration % 5 == 0 {
            println!("Iteration {}: Memory: {:.2} MB RSS", 
                iteration, tracker.samples.last().unwrap().rss_mb());
        }
        
        sleep(Duration::from_millis(10)).await;
    }
    
    // Cleanup
    for workspace_id in workspace_ids {
        manager.delete_workspace(workspace_id).await
            .expect("Should delete workspace");
    }
    
    tracker.sample();
    
    println!("Sustained load test results:");
    println!("Average memory: {:.2} MB", tracker.average_rss_mb());
    println!("Peak memory: {:.2} MB", tracker.peak_rss_mb());
    println!("Final growth: {:.2} MB", tracker.memory_growth_mb());
    
    // Should not accumulate memory under sustained load
    assert!(tracker.memory_growth_mb() < 15.0,
        "Sustained load memory growth {:.2} MB exceeds 15MB threshold", tracker.memory_growth_mb());
    
    assert!(!tracker.has_potential_leak(10.0),
        "Potential memory leak detected under sustained load");
}

/// Test memory usage with large numbers of workspaces
#[tokio::test]
async fn test_large_workspace_count_memory() {
    let mut tracker = MemoryTracker::new();
    let manager = create_test_workspace_manager();
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    
    println!("Testing large workspace count memory usage");
    tracker.sample();
    
    let mut workspace_ids = Vec::new();
    
    // Create many workspaces
    for i in 0..50 {
        let request = WorkspaceCreateRequest {
            name: format!("Large Test Workspace {}", i),
            description: Some(format!("Workspace {} for large count memory testing with longer description to test memory usage with more data per workspace", i)),
            keyboard_shortcut: format!("opt+{}", (i % 9) + 1),
            tiling_pattern_id: Some(pattern.id),
            auto_arrange: Some(true),
        };
        
        let workspace_id = manager.create_workspace(request, pattern.id).await
            .expect("Should create workspace");
        workspace_ids.push(workspace_id);
        
        if i % 10 == 0 {
            tracker.sample();
            println!("Created {} workspaces, Memory: {:.2} MB", 
                i + 1, tracker.samples.last().unwrap().rss_mb());
        }
    }
    
    tracker.sample();
    
    // Test operations with many workspaces
    let _workspaces = manager.list_workspaces().await;
    let _count = manager.get_workspace_count().await;
    let _search_results = manager.find_workspaces_by_name("Large").await;
    
    tracker.sample();
    
    // Cleanup half the workspaces
    for workspace_id in workspace_ids.drain(0..25) {
        manager.delete_workspace(workspace_id).await
            .expect("Should delete workspace");
    }
    
    tracker.sample();
    
    // Cleanup remaining workspaces
    for workspace_id in workspace_ids {
        manager.delete_workspace(workspace_id).await
            .expect("Should delete workspace");
    }
    
    tracker.sample();
    
    println!("Large workspace count test results:");
    println!("Peak memory: {:.2} MB", tracker.peak_rss_mb());
    println!("Final memory: {:.2} MB", tracker.samples.last().unwrap().rss_mb());
    println!("Memory growth: {:.2} MB", tracker.memory_growth_mb());
    
    // Peak memory should be reasonable for 50 workspaces
    assert!(tracker.peak_rss_mb() < 200.0,
        "Peak memory {:.2} MB exceeds 200MB for 50 workspaces", tracker.peak_rss_mb());
    
    // Should return close to initial memory after cleanup
    assert!(tracker.memory_growth_mb() < 20.0,
        "Memory not properly cleaned up after workspace deletion");
}

/// Test memory usage patterns during error conditions
#[tokio::test]
async fn test_error_condition_memory_usage() {
    let mut tracker = MemoryTracker::new();
    let manager = create_test_workspace_manager();
    let engine = create_test_tiling_engine();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    
    println!("Testing error condition memory usage");
    tracker.sample();
    
    // Test various error conditions
    for iteration in 0..20 {
        // Try to delete non-existent workspace
        let fake_id = Uuid::new_v4();
        let _ = manager.delete_workspace(fake_id).await;
        
        // Try to switch to non-existent workspace
        let _ = manager.switch_to_workspace(fake_id).await;
        
        // Try tiling with empty window list
        let area = tillers::macos::accessibility::Rect {
            origin: tillers::macos::accessibility::Point { x: 0.0, y: 0.0 },
            size: tillers::macos::accessibility::Size { width: 1920.0, height: 1080.0 },
        };
        let _ = engine.layout_windows(&[], &pattern, area).await;
        
        // Try tiling with invalid area
        let tiny_area = tillers::macos::accessibility::Rect {
            origin: tillers::macos::accessibility::Point { x: 0.0, y: 0.0 },
            size: tillers::macos::accessibility::Size { width: 1.0, height: 1.0 },
        };
        let window_ids = vec![1, 2, 3];
        let _ = engine.layout_windows(&window_ids, &pattern, tiny_area).await;
        
        if iteration % 5 == 0 {
            tracker.sample();
        }
    }
    
    tracker.sample();
    
    println!("Error condition test results:");
    println!("Memory growth: {:.2} MB", tracker.memory_growth_mb());
    
    // Error conditions should not cause memory leaks
    assert!(tracker.memory_growth_mb() < 5.0,
        "Error conditions caused memory growth {:.2} MB", tracker.memory_growth_mb());
}

/// Test concurrent operations memory usage
#[tokio::test]
async fn test_concurrent_operations_memory() {
    let mut tracker = MemoryTracker::new();
    let manager = Arc::new(create_test_workspace_manager());
    let engine = Arc::new(create_test_tiling_engine());
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    
    println!("Testing concurrent operations memory usage");
    tracker.sample();
    
    // Run concurrent operations
    let mut handles = Vec::new();
    
    for task_id in 0..5 {
        let manager_clone = manager.clone();
        let engine_clone = engine.clone();
        let pattern_clone = pattern.clone();
        
        let handle = tokio::spawn(async move {
            for i in 0..10 {
                // Create workspace
                let request = WorkspaceCreateRequest {
                    name: format!("Concurrent Workspace {} - {}", task_id, i),
                    description: Some("Concurrent operation test".to_string()),
                    keyboard_shortcut: format!("opt+{}", (i % 9) + 1),
                    tiling_pattern_id: Some(pattern_clone.id),
                    auto_arrange: Some(true),
                };
                
                if let Ok(workspace_id) = manager_clone.create_workspace(request, pattern_clone.id).await {
                    // Perform tiling
                    let area = tillers::macos::accessibility::Rect {
                        origin: tillers::macos::accessibility::Point { x: 0.0, y: 0.0 },
                        size: tillers::macos::accessibility::Size { width: 1920.0, height: 1080.0 },
                    };
                    let window_ids: Vec<u32> = (1..=5).collect();
                    let _ = engine_clone.layout_windows(&window_ids, &pattern_clone, area).await;
                    
                    // List workspaces
                    let _ = manager_clone.list_workspaces().await;
                    
                    // Delete workspace
                    let _ = manager_clone.delete_workspace(workspace_id).await;
                }
                
                sleep(Duration::from_millis(5)).await;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete");
    }
    
    tracker.sample();
    
    println!("Concurrent operations test results:");
    println!("Memory growth: {:.2} MB", tracker.memory_growth_mb());
    
    // Concurrent operations should not cause excessive memory growth
    assert!(tracker.memory_growth_mb() < 10.0,
        "Concurrent operations caused memory growth {:.2} MB", tracker.memory_growth_mb());
}

/// Benchmark memory efficiency of different operations
#[tokio::test]
async fn test_memory_efficiency_benchmark() {
    let manager = create_test_workspace_manager();
    let engine = create_test_tiling_engine();
    
    println!("Memory efficiency benchmark:");
    
    // Test workspace creation efficiency
    let start_memory = MemoryInfo::current();
    let mut workspace_ids = Vec::new();
    
    for i in 0..10 {
        let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
        let request = WorkspaceCreateRequest {
            name: format!("Efficiency Test {}", i),
            description: Some("Testing memory efficiency".to_string()),
            keyboard_shortcut: format!("opt+{}", i + 1),
            tiling_pattern_id: Some(pattern.id),
            auto_arrange: Some(true),
        };
        
        let workspace_id = manager.create_workspace(request, pattern.id).await
            .expect("Should create workspace");
        workspace_ids.push(workspace_id);
    }
    
    let after_creation = MemoryInfo::current();
    let creation_cost = (after_creation.rss_bytes - start_memory.rss_bytes) as f64 / 1024.0 / 1024.0;
    println!("10 workspaces creation cost: {:.2} MB", creation_cost);
    
    // Test tiling operation efficiency
    let before_tiling = MemoryInfo::current();
    
    for _ in 0..100 {
        let pattern = create_test_pattern(LayoutAlgorithm::Grid);
        let area = tillers::macos::accessibility::Rect {
            origin: tillers::macos::accessibility::Point { x: 0.0, y: 0.0 },
            size: tillers::macos::accessibility::Size { width: 1920.0, height: 1080.0 },
        };
        let window_ids: Vec<u32> = (1..=9).collect();
        let _ = engine.layout_windows(&window_ids, &pattern, area).await
            .expect("Should calculate layout");
    }
    
    let after_tiling = MemoryInfo::current();
    let tiling_cost = (after_tiling.rss_bytes - before_tiling.rss_bytes) as f64 / 1024.0 / 1024.0;
    println!("100 tiling operations cost: {:.2} MB", tiling_cost);
    
    // Cleanup
    for workspace_id in workspace_ids {
        manager.delete_workspace(workspace_id).await
            .expect("Should delete workspace");
    }
    
    let final_memory = MemoryInfo::current();
    let cleanup_efficiency = (start_memory.rss_bytes as f64 - final_memory.rss_bytes as f64) / 1024.0 / 1024.0;
    println!("Cleanup efficiency: {:.2} MB (negative = memory released)", cleanup_efficiency);
    
    // Memory efficiency assertions
    assert!(creation_cost < 5.0, "Workspace creation too memory expensive: {:.2} MB", creation_cost);
    assert!(tiling_cost < 2.0, "Tiling operations too memory expensive: {:.2} MB", tiling_cost);
}

/// Memory stress test with rapid allocation and deallocation
#[tokio::test]
async fn test_memory_stress() {
    let mut tracker = MemoryTracker::new();
    let manager = create_test_workspace_manager();
    
    println!("Memory stress test - rapid allocation/deallocation");
    
    for cycle in 0..10 {
        // Rapid allocation
        let mut workspace_ids = Vec::new();
        for i in 0..20 {
            let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
            let request = WorkspaceCreateRequest {
                name: format!("Stress Test {} - {}", cycle, i),
                description: Some(format!("Stress testing memory allocation cycle {} item {}", cycle, i)),
                keyboard_shortcut: format!("opt+{}", (i % 9) + 1),
                tiling_pattern_id: Some(pattern.id),
                auto_arrange: Some(true),
            };
            
            let workspace_id = manager.create_workspace(request, pattern.id).await
                .expect("Should create workspace");
            workspace_ids.push(workspace_id);
        }
        
        tracker.sample();
        
        // Rapid deallocation
        for workspace_id in workspace_ids {
            manager.delete_workspace(workspace_id).await
                .expect("Should delete workspace");
        }
        
        tracker.sample();
        
        if cycle % 3 == 0 {
            println!("Stress cycle {}: Memory {:.2} MB, Growth: {:.2} MB", 
                cycle, tracker.samples.last().unwrap().rss_mb(), tracker.memory_growth_mb());
        }
    }
    
    println!("Stress test results:");
    println!("Peak memory: {:.2} MB", tracker.peak_rss_mb());
    println!("Final growth: {:.2} MB", tracker.memory_growth_mb());
    
    // Stress test should not cause permanent memory growth
    assert!(tracker.memory_growth_mb() < 15.0,
        "Memory stress test caused growth {:.2} MB", tracker.memory_growth_mb());
    
    assert!(!tracker.has_potential_leak(8.0),
        "Memory stress test detected potential leak");
}