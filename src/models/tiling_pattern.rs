use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Algorithm types for window layout
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LayoutAlgorithm {
    MasterStack,
    Grid,
    Columns,
    Custom,
}

/// How pattern adapts to new windows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResizeBehavior {
    Shrink,   // Reduce window sizes to fit new windows
    Stack,    // Stack new windows in overflow area
    Overflow, // Allow windows to extend beyond visible area
}

/// Represents a window tiling pattern with layout algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilingPattern {
    /// Unique identifier
    pub id: Uuid,
    /// Pattern name (e.g., "Two Column", "Master-Stack", "Grid")
    pub name: String,
    /// Algorithm type for layout calculation
    pub layout_algorithm: LayoutAlgorithm,
    /// Percentage of screen for main window area (0.0-1.0)
    pub main_area_ratio: f64,
    /// Pixel gap between windows
    pub gap_size: u32,
    /// Pixel margin around windows
    pub window_margin: u32,
    /// Maximum windows before pattern adjustment
    pub max_windows: u32,
    /// How pattern adapts to new windows
    pub resize_behavior: ResizeBehavior,
}

/// Rectangle representing window position and size
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Computed layout for windows within a pattern
#[derive(Debug, Clone)]
pub struct ComputedLayout {
    pub window_rects: Vec<WindowRect>,
    pub main_area: WindowRect,
    pub overflow_area: Option<WindowRect>,
}

impl TilingPattern {
    /// Create a new tiling pattern with validation
    pub fn new(
        name: String,
        layout_algorithm: LayoutAlgorithm,
        main_area_ratio: f64,
        gap_size: u32,
        window_margin: u32,
        max_windows: u32,
        resize_behavior: ResizeBehavior,
    ) -> Result<Self, TilingPatternError> {
        // Validate main area ratio
        if !(0.1..=0.9).contains(&main_area_ratio) {
            return Err(TilingPatternError::InvalidMainAreaRatio(main_area_ratio));
        }

        // Validate max windows
        if max_windows == 0 {
            return Err(TilingPatternError::InvalidMaxWindows(max_windows));
        }

        Ok(TilingPattern {
            id: Uuid::new_v4(),
            name,
            layout_algorithm,
            main_area_ratio,
            gap_size,
            window_margin,
            max_windows,
            resize_behavior,
        })
    }

    /// Compute window layout for given screen area and window count
    pub fn compute_layout(
        &self,
        screen_area: &WindowRect,
        window_count: usize,
    ) -> Result<ComputedLayout, TilingPatternError> {
        if window_count == 0 {
            return Ok(ComputedLayout {
                window_rects: vec![],
                main_area: screen_area.clone(),
                overflow_area: None,
            });
        }

        let effective_area = WindowRect {
            x: screen_area.x + self.window_margin as i32,
            y: screen_area.y + self.window_margin as i32,
            width: screen_area.width - 2 * self.window_margin,
            height: screen_area.height - 2 * self.window_margin,
        };

        match self.layout_algorithm {
            LayoutAlgorithm::MasterStack => {
                self.compute_master_stack_layout(&effective_area, window_count)
            }
            LayoutAlgorithm::Grid => self.compute_grid_layout(&effective_area, window_count),
            LayoutAlgorithm::Columns => self.compute_columns_layout(&effective_area, window_count),
            LayoutAlgorithm::Custom => self.compute_custom_layout(&effective_area, window_count),
        }
    }

    /// Compute master-stack layout (one main window, others stacked)
    fn compute_master_stack_layout(
        &self,
        area: &WindowRect,
        window_count: usize,
    ) -> Result<ComputedLayout, TilingPatternError> {
        let mut window_rects = Vec::new();

        if window_count == 1 {
            // Single window takes full area
            window_rects.push(area.clone());
        } else {
            // Calculate main area and stack area
            let main_width = (area.width as f64 * self.main_area_ratio) as u32;
            let stack_width = area.width - main_width - self.gap_size;

            // Main window
            let main_rect = WindowRect {
                x: area.x,
                y: area.y,
                width: main_width,
                height: area.height,
            };
            window_rects.push(main_rect.clone());

            // Stack windows
            let stack_window_count = window_count - 1;
            let stack_window_height = if stack_window_count > 0 {
                (area.height - (stack_window_count - 1) as u32 * self.gap_size)
                    / stack_window_count as u32
            } else {
                0
            };

            for i in 0..stack_window_count {
                let stack_rect = WindowRect {
                    x: area.x + main_width as i32 + self.gap_size as i32,
                    y: area.y + (i as u32 * (stack_window_height + self.gap_size)) as i32,
                    width: stack_width,
                    height: stack_window_height,
                };
                window_rects.push(stack_rect);
            }
        }

        Ok(ComputedLayout {
            window_rects,
            main_area: area.clone(),
            overflow_area: None,
        })
    }

    /// Compute grid layout (windows arranged in grid)
    fn compute_grid_layout(
        &self,
        area: &WindowRect,
        window_count: usize,
    ) -> Result<ComputedLayout, TilingPatternError> {
        let mut window_rects = Vec::new();

        // Calculate grid dimensions
        let cols = (window_count as f64).sqrt().ceil() as u32;
        let rows = (window_count as f64 / cols as f64).ceil() as u32;

        let window_width = (area.width - (cols - 1) * self.gap_size) / cols;
        let window_height = (area.height - (rows - 1) * self.gap_size) / rows;

        for i in 0..window_count {
            let row = i as u32 / cols;
            let col = i as u32 % cols;

            let rect = WindowRect {
                x: area.x + (col * (window_width + self.gap_size)) as i32,
                y: area.y + (row * (window_height + self.gap_size)) as i32,
                width: window_width,
                height: window_height,
            };
            window_rects.push(rect);
        }

        Ok(ComputedLayout {
            window_rects,
            main_area: area.clone(),
            overflow_area: None,
        })
    }

    /// Compute columns layout (windows arranged in vertical columns)
    fn compute_columns_layout(
        &self,
        area: &WindowRect,
        window_count: usize,
    ) -> Result<ComputedLayout, TilingPatternError> {
        let mut window_rects = Vec::new();

        // Each window gets equal column width
        let window_width =
            (area.width - (window_count - 1) as u32 * self.gap_size) / window_count as u32;

        for i in 0..window_count {
            let rect = WindowRect {
                x: area.x + (i as u32 * (window_width + self.gap_size)) as i32,
                y: area.y,
                width: window_width,
                height: area.height,
            };
            window_rects.push(rect);
        }

        Ok(ComputedLayout {
            window_rects,
            main_area: area.clone(),
            overflow_area: None,
        })
    }

    /// Compute custom layout (placeholder for user-defined layouts)
    fn compute_custom_layout(
        &self,
        area: &WindowRect,
        window_count: usize,
    ) -> Result<ComputedLayout, TilingPatternError> {
        // For now, fallback to grid layout
        // TODO: Implement custom layout definition system
        self.compute_grid_layout(area, window_count)
    }

    /// Validate the tiling pattern configuration
    pub fn validate(&self) -> Result<(), TilingPatternError> {
        if !(0.1..=0.9).contains(&self.main_area_ratio) {
            return Err(TilingPatternError::InvalidMainAreaRatio(
                self.main_area_ratio,
            ));
        }

        if self.max_windows == 0 {
            return Err(TilingPatternError::InvalidMaxWindows(self.max_windows));
        }

        if self.name.trim().is_empty() {
            return Err(TilingPatternError::EmptyName);
        }

        Ok(())
    }
}

/// Errors that can occur with tiling patterns
#[derive(Debug, thiserror::Error)]
pub enum TilingPatternError {
    #[error("Invalid main area ratio: {0}. Must be between 0.1 and 0.9")]
    InvalidMainAreaRatio(f64),

    #[error("Invalid max windows: {0}. Must be greater than 0")]
    InvalidMaxWindows(u32),

    #[error("Pattern name cannot be empty")]
    EmptyName,

    #[error("Invalid screen area dimensions")]
    InvalidScreenArea,

    #[error("Layout computation failed: {0}")]
    LayoutComputationFailed(String),
}

impl Default for TilingPattern {
    fn default() -> Self {
        TilingPattern {
            id: Uuid::new_v4(),
            name: "Default".to_string(),
            layout_algorithm: LayoutAlgorithm::MasterStack,
            main_area_ratio: 0.6,
            gap_size: 10,
            window_margin: 10,
            max_windows: 10,
            resize_behavior: ResizeBehavior::Shrink,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tiling_pattern_valid() {
        let pattern = TilingPattern::new(
            "Test Pattern".to_string(),
            LayoutAlgorithm::MasterStack,
            0.6,
            10,
            5,
            8,
            ResizeBehavior::Shrink,
        );

        assert!(pattern.is_ok());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.name, "Test Pattern");
        assert_eq!(pattern.main_area_ratio, 0.6);
    }

    #[test]
    fn test_new_tiling_pattern_invalid_ratio() {
        let pattern = TilingPattern::new(
            "Test".to_string(),
            LayoutAlgorithm::MasterStack,
            1.5, // Invalid ratio
            10,
            5,
            8,
            ResizeBehavior::Shrink,
        );

        assert!(pattern.is_err());
    }

    #[test]
    fn test_master_stack_layout_single_window() {
        let pattern = TilingPattern::default();
        let screen_area = WindowRect {
            x: 0,
            y: 0,
            width: 1000,
            height: 800,
        };

        let layout = pattern.compute_layout(&screen_area, 1).unwrap();
        assert_eq!(layout.window_rects.len(), 1);

        // Single window should use available area minus margins
        let expected_rect = WindowRect {
            x: 10,
            y: 10,
            width: 980,
            height: 780,
        };
        assert_eq!(layout.window_rects[0], expected_rect);
    }

    #[test]
    fn test_master_stack_layout_multiple_windows() {
        let pattern = TilingPattern::default();
        let screen_area = WindowRect {
            x: 0,
            y: 0,
            width: 1000,
            height: 800,
        };

        let layout = pattern.compute_layout(&screen_area, 3).unwrap();
        assert_eq!(layout.window_rects.len(), 3);

        // Main window should use 60% of width
        let main_rect = &layout.window_rects[0];
        assert_eq!(main_rect.width, 588); // (980 * 0.6) rounded
        assert_eq!(main_rect.height, 780);
    }

    #[test]
    fn test_grid_layout() {
        let pattern = TilingPattern {
            layout_algorithm: LayoutAlgorithm::Grid,
            ..Default::default()
        };
        let screen_area = WindowRect {
            x: 0,
            y: 0,
            width: 1000,
            height: 800,
        };

        let layout = pattern.compute_layout(&screen_area, 4).unwrap();
        assert_eq!(layout.window_rects.len(), 4);

        // 4 windows should form 2x2 grid
        assert_eq!(layout.window_rects[0].width, layout.window_rects[1].width);
        assert_eq!(layout.window_rects[0].height, layout.window_rects[2].height);
    }

    #[test]
    fn test_columns_layout() {
        let pattern = TilingPattern {
            layout_algorithm: LayoutAlgorithm::Columns,
            ..Default::default()
        };
        let screen_area = WindowRect {
            x: 0,
            y: 0,
            width: 1000,
            height: 800,
        };

        let layout = pattern.compute_layout(&screen_area, 3).unwrap();
        assert_eq!(layout.window_rects.len(), 3);

        // All windows should have same width and full height
        for rect in &layout.window_rects {
            assert_eq!(rect.height, 780); // Full height minus margins
        }
    }

    #[test]
    fn test_validation() {
        let mut pattern = TilingPattern::default();
        assert!(pattern.validate().is_ok());

        pattern.main_area_ratio = 1.5;
        assert!(pattern.validate().is_err());

        pattern.main_area_ratio = 0.6;
        pattern.max_windows = 0;
        assert!(pattern.validate().is_err());

        pattern.max_windows = 10;
        pattern.name = "".to_string();
        assert!(pattern.validate().is_err());
    }
}
