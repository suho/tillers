//! Unit tests for tiling algorithms
//!
//! Tests the core tiling engine algorithms for window layout calculation,
//! ensuring proper geometry, edge cases, and performance characteristics.

use tillers::{
    macos::accessibility::{Point, Rect, Size},
    models::tiling_pattern::{LayoutAlgorithm, ResizeBehavior, TilingPattern},
    services::tiling_engine::{TilingEngine, WindowLayout},
};
use uuid::Uuid;

/// Create a standard test pattern with default values
fn create_test_pattern(algorithm: LayoutAlgorithm) -> TilingPattern {
    TilingPattern {
        id: Uuid::new_v4(),
        name: format!("{:?} Test Pattern", algorithm),
        layout_algorithm: algorithm,
        main_area_ratio: 0.6,
        gap_size: 10,
        window_margin: 20,
        max_windows: 10,
        resize_behavior: ResizeBehavior::Shrink,
    }
}

/// Create a standard test area (1920x1080 screen)
fn create_test_area() -> Rect {
    Rect {
        origin: Point { x: 0.0, y: 0.0 },
        size: Size {
            width: 1920.0,
            height: 1080.0,
        },
    }
}

/// Create a smaller test area for edge case testing
fn create_small_area() -> Rect {
    Rect {
        origin: Point { x: 100.0, y: 100.0 },
        size: Size {
            width: 400.0,
            height: 300.0,
        },
    }
}

/// Assert that a rect is within expected bounds and has reasonable dimensions
fn assert_rect_valid(rect: &Rect, area: &Rect, min_width: f64, min_height: f64) {
    assert!(rect.origin.x >= area.origin.x, "Rect x origin {} should be >= area x origin {}", rect.origin.x, area.origin.x);
    assert!(rect.origin.y >= area.origin.y, "Rect y origin {} should be >= area y origin {}", rect.origin.y, area.origin.y);
    assert!(rect.origin.x + rect.size.width <= area.origin.x + area.size.width, 
        "Rect right edge {} should be <= area right edge {}", 
        rect.origin.x + rect.size.width, area.origin.x + area.size.width);
    assert!(rect.origin.y + rect.size.height <= area.origin.y + area.size.height,
        "Rect bottom edge {} should be <= area bottom edge {}",
        rect.origin.y + rect.size.height, area.origin.y + area.size.height);
    assert!(rect.size.width >= min_width, "Rect width {} should be >= {}", rect.size.width, min_width);
    assert!(rect.size.height >= min_height, "Rect height {} should be >= {}", rect.size.height, min_height);
}

/// Check that window layouts don't overlap (with tolerance for gaps)
fn assert_no_overlaps(layouts: &[WindowLayout], gap_tolerance: f64) {
    for (i, layout1) in layouts.iter().enumerate() {
        for (j, layout2) in layouts.iter().enumerate() {
            if i >= j {
                continue; // Only check each pair once
            }
            
            let rect1 = &layout1.frame;
            let rect2 = &layout2.frame;
            
            // Check if rectangles overlap (accounting for gap tolerance)
            let no_x_overlap = rect1.origin.x + rect1.size.width + gap_tolerance <= rect2.origin.x ||
                               rect2.origin.x + rect2.size.width + gap_tolerance <= rect1.origin.x;
            let no_y_overlap = rect1.origin.y + rect1.size.height + gap_tolerance <= rect2.origin.y ||
                               rect2.origin.y + rect2.size.height + gap_tolerance <= rect1.origin.y;
            
            assert!(no_x_overlap || no_y_overlap, 
                "Windows {} and {} overlap: rect1=({}, {}, {}, {}), rect2=({}, {}, {}, {})",
                layout1.window_id, layout2.window_id,
                rect1.origin.x, rect1.origin.y, rect1.size.width, rect1.size.height,
                rect2.origin.x, rect2.origin.y, rect2.size.width, rect2.size.height);
        }
    }
}

#[tokio::test]
async fn test_tiling_engine_creation() {
    let engine = TilingEngine::new();
    let metrics = engine.metrics().await;
    
    assert_eq!(metrics.layout_requests, 0);
    assert_eq!(metrics.last_window_count, 0);
    assert!(metrics.last_algorithm.is_none());
}

#[tokio::test]
async fn test_master_stack_single_window() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    let area = create_test_area();
    let window_ids = vec![1];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].window_id, 1);
    
    // Single window should fill most of the area (minus margins)
    let expected_width = area.size.width - 2.0 * pattern.window_margin as f64;
    let expected_height = area.size.height - 2.0 * pattern.window_margin as f64;
    
    assert_rect_valid(&layouts[0].frame, &area, expected_width * 0.9, expected_height * 0.9);
}

#[tokio::test]
async fn test_master_stack_two_windows() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    let area = create_test_area();
    let window_ids = vec![1, 2];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 2);
    assert_no_overlaps(&layouts, pattern.gap_size as f64);
    
    // First window (master) should be larger
    let master = &layouts[0].frame;
    let stack = &layouts[1].frame;
    
    assert!(master.size.width > stack.size.width, 
        "Master window width {} should be > stack window width {}", 
        master.size.width, stack.size.width);
    
    // Both should have valid dimensions
    assert_rect_valid(master, &area, 100.0, 100.0);
    assert_rect_valid(stack, &area, 50.0, 100.0);
}

#[tokio::test]
async fn test_master_stack_multiple_windows() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    let area = create_test_area();
    let window_ids = vec![1, 2, 3, 4];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 4);
    assert_no_overlaps(&layouts, pattern.gap_size as f64);
    
    // Master window should be first
    let master = &layouts[0].frame;
    
    // Stack windows should be vertically arranged
    for i in 1..layouts.len() {
        assert_rect_valid(&layouts[i].frame, &area, 50.0, 50.0);
        
        // Each stack window should be below the previous one
        if i > 1 {
            assert!(layouts[i].frame.origin.y >= layouts[i-1].frame.origin.y,
                "Stack window {} y origin {} should be >= previous window y origin {}",
                i, layouts[i].frame.origin.y, layouts[i-1].frame.origin.y);
        }
    }
}

#[tokio::test]
async fn test_grid_layout_single_window() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    let area = create_test_area();
    let window_ids = vec![1];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 1);
    assert_rect_valid(&layouts[0].frame, &area, 100.0, 100.0);
}

#[tokio::test]
async fn test_grid_layout_four_windows() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    let area = create_test_area();
    let window_ids = vec![1, 2, 3, 4];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 4);
    assert_no_overlaps(&layouts, pattern.gap_size as f64);
    
    // In a 2x2 grid, all windows should have similar dimensions
    let first_width = layouts[0].frame.size.width;
    let first_height = layouts[0].frame.size.height;
    
    for layout in &layouts {
        assert_rect_valid(&layout.frame, &area, 100.0, 100.0);
        
        // Allow some tolerance for rounding
        assert!((layout.frame.size.width - first_width).abs() < 2.0,
            "Grid window width {} should be approximately {}", 
            layout.frame.size.width, first_width);
        assert!((layout.frame.size.height - first_height).abs() < 2.0,
            "Grid window height {} should be approximately {}", 
            layout.frame.size.height, first_height);
    }
}

#[tokio::test]
async fn test_grid_layout_odd_windows() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    let area = create_test_area();
    let window_ids = vec![1, 2, 3, 4, 5];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 5);
    assert_no_overlaps(&layouts, pattern.gap_size as f64);
    
    // All windows should fit within the area
    for layout in &layouts {
        assert_rect_valid(&layout.frame, &area, 50.0, 50.0);
    }
}

#[tokio::test]
async fn test_columns_layout() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Columns);
    let area = create_test_area();
    let window_ids = vec![1, 2, 3];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 3);
    assert_no_overlaps(&layouts, pattern.gap_size as f64);
    
    // Column layout should arrange windows side by side
    for i in 1..layouts.len() {
        assert!(layouts[i].frame.origin.x >= layouts[i-1].frame.origin.x,
            "Column window {} x origin {} should be >= previous window x origin {}",
            i, layouts[i].frame.origin.x, layouts[i-1].frame.origin.x);
    }
    
    // All windows should have similar heights (full height minus margins)
    let expected_height = area.size.height - 2.0 * pattern.window_margin as f64;
    for layout in &layouts {
        assert!((layout.frame.size.height - expected_height).abs() < 2.0,
            "Column window height {} should be approximately {}", 
            layout.frame.size.height, expected_height);
    }
}

#[tokio::test]
async fn test_custom_layout() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Custom);
    let area = create_test_area();
    let window_ids = vec![1, 2];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 2);
    assert_no_overlaps(&layouts, 5.0); // Custom layout may have smaller gaps
    
    for layout in &layouts {
        assert_rect_valid(&layout.frame, &area, 50.0, 50.0);
    }
}

#[tokio::test]
async fn test_empty_window_list() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    let area = create_test_area();
    let window_ids = vec![];
    
    let result = engine.layout_windows(&window_ids, &pattern, area).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("At least one window"));
}

#[tokio::test]
async fn test_small_area_handling() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    let area = create_small_area();
    let window_ids = vec![1, 2];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 2);
    
    for layout in &layouts {
        assert_rect_valid(&layout.frame, &area, 10.0, 10.0);
    }
}

#[tokio::test]
async fn test_large_margin_handling() {
    let engine = TilingEngine::new();
    let mut pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    pattern.window_margin = 100; // Large margin
    
    let area = create_test_area();
    let window_ids = vec![1];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 1);
    
    // Window should still fit within the available area
    let layout = &layouts[0];
    assert!(layout.frame.origin.x >= area.origin.x + pattern.window_margin as f64);
    assert!(layout.frame.origin.y >= area.origin.y + pattern.window_margin as f64);
    assert!(layout.frame.origin.x + layout.frame.size.width <= 
            area.origin.x + area.size.width - pattern.window_margin as f64);
    assert!(layout.frame.origin.y + layout.frame.size.height <= 
            area.origin.y + area.size.height - pattern.window_margin as f64);
}

#[tokio::test]
async fn test_large_gap_handling() {
    let engine = TilingEngine::new();
    let mut pattern = create_test_pattern(LayoutAlgorithm::Grid);
    pattern.gap_size = 50; // Large gap
    
    let area = create_test_area();
    let window_ids = vec![1, 2, 3, 4];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 4);
    assert_no_overlaps(&layouts, pattern.gap_size as f64);
    
    // All windows should still have reasonable sizes despite large gaps
    for layout in &layouts {
        assert!(layout.frame.size.width > 50.0, 
            "Window width {} should be > 50 despite large gaps", layout.frame.size.width);
        assert!(layout.frame.size.height > 50.0,
            "Window height {} should be > 50 despite large gaps", layout.frame.size.height);
    }
}

#[tokio::test]
async fn test_main_area_ratio_extremes() {
    let engine = TilingEngine::new();
    
    // Test very small main area ratio
    let mut pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    pattern.main_area_ratio = 0.1;
    
    let area = create_test_area();
    let window_ids = vec![1, 2];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    assert_eq!(layouts.len(), 2);
    
    // Master window should still have minimum width (40% as per implementation)
    let master = &layouts[0].frame;
    let expected_min_width = (area.size.width - 2.0 * pattern.window_margin as f64) * 0.4;
    assert!(master.size.width >= expected_min_width - 1.0, // Allow small tolerance
        "Master window width {} should be >= minimum {}", master.size.width, expected_min_width);
    
    // Test very large main area ratio
    pattern.main_area_ratio = 0.95;
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    assert_eq!(layouts.len(), 2);
    
    // Stack window should still have some width
    let stack = &layouts[1].frame;
    assert!(stack.size.width > 10.0, 
        "Stack window width {} should be > 10 even with large main ratio", stack.size.width);
}

#[tokio::test]
async fn test_many_windows() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    let area = create_test_area();
    let window_ids: Vec<u32> = (1..=16).collect(); // 16 windows
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 16);
    assert_no_overlaps(&layouts, pattern.gap_size as f64);
    
    // All windows should fit and have reasonable minimum sizes
    for layout in &layouts {
        assert_rect_valid(&layout.frame, &area, 20.0, 20.0);
    }
}

#[tokio::test]
async fn test_metrics_tracking() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::MasterStack);
    let area = create_test_area();
    
    // Initial metrics
    let initial_metrics = engine.metrics().await;
    assert_eq!(initial_metrics.layout_requests, 0);
    
    // Perform layout operation
    let window_ids = vec![1, 2, 3];
    let _layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    // Check updated metrics
    let updated_metrics = engine.metrics().await;
    assert_eq!(updated_metrics.layout_requests, 1);
    assert_eq!(updated_metrics.last_window_count, 3);
    assert_eq!(updated_metrics.last_algorithm, Some(LayoutAlgorithm::MasterStack));
    
    // Perform another operation
    let window_ids = vec![1, 2, 3, 4, 5];
    let _layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    let final_metrics = engine.metrics().await;
    assert_eq!(final_metrics.layout_requests, 2);
    assert_eq!(final_metrics.last_window_count, 5);
}

#[tokio::test]
async fn test_window_id_preservation() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Grid);
    let area = create_test_area();
    let window_ids = vec![42, 100, 999];
    
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 3);
    
    // Check that window IDs are preserved in correct order
    assert_eq!(layouts[0].window_id, 42);
    assert_eq!(layouts[1].window_id, 100);
    assert_eq!(layouts[2].window_id, 999);
}

#[tokio::test]
async fn test_area_origin_offset() {
    let engine = TilingEngine::new();
    let pattern = create_test_pattern(LayoutAlgorithm::Columns);
    
    // Test with non-zero origin
    let area = Rect {
        origin: Point { x: 500.0, y: 300.0 },
        size: Size { width: 800.0, height: 600.0 },
    };
    
    let window_ids = vec![1, 2];
    let layouts = engine.layout_windows(&window_ids, &pattern, area).await.unwrap();
    
    assert_eq!(layouts.len(), 2);
    
    // All windows should be positioned relative to the area origin
    for layout in &layouts {
        assert!(layout.frame.origin.x >= area.origin.x,
            "Window x {} should be >= area origin x {}", layout.frame.origin.x, area.origin.x);
        assert!(layout.frame.origin.y >= area.origin.y,
            "Window y {} should be >= area origin y {}", layout.frame.origin.y, area.origin.y);
    }
}