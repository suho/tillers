use crate::macos::accessibility::{Point, Rect, Size};
use crate::models::tiling_pattern::{LayoutAlgorithm, TilingPattern};
use crate::{Result, TilleRSError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Layout assignment returned by the tiling engine
#[derive(Debug, Clone, PartialEq)]
pub struct WindowLayout {
    pub window_id: u32,
    pub frame: Rect,
}

/// Metrics captured for tiling operations
#[derive(Debug, Default, Clone)]
pub struct TilingEngineMetrics {
    pub layout_requests: u64,
    pub last_window_count: usize,
    pub last_algorithm: Option<LayoutAlgorithm>,
}

/// Calculates window layouts based on tiling patterns
pub struct TilingEngine {
    metrics: Arc<RwLock<TilingEngineMetrics>>,
}

impl Default for TilingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TilingEngine {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(TilingEngineMetrics::default())),
        }
    }

    /// Compute window frames for the supplied IDs using the pattern and available area
    pub async fn layout_windows(
        &self,
        window_ids: &[u32],
        pattern: &TilingPattern,
        available_area: Rect,
    ) -> Result<Vec<WindowLayout>> {
        if window_ids.is_empty() {
            return Err(TilleRSError::ValidationError(
                "At least one window is required for tiling".into(),
            )
            .into());
        }

        let frames = match pattern.layout_algorithm {
            LayoutAlgorithm::MasterStack => {
                master_stack_layout(window_ids.len(), pattern, available_area)?
            }
            LayoutAlgorithm::Grid => grid_layout(window_ids.len(), pattern, available_area)?,
            LayoutAlgorithm::Columns => columns_layout(window_ids.len(), pattern, available_area)?,
            LayoutAlgorithm::Custom => custom_fallback(window_ids.len(), available_area)?,
        };

        let result: Vec<_> = window_ids
            .iter()
            .cloned()
            .zip(frames.into_iter())
            .map(|(window_id, frame)| WindowLayout { window_id, frame })
            .collect();

        let mut metrics = self.metrics.write().await;
        metrics.layout_requests += 1;
        metrics.last_window_count = window_ids.len();
        metrics.last_algorithm = Some(pattern.layout_algorithm.clone());

        Ok(result)
    }

    pub async fn metrics(&self) -> TilingEngineMetrics {
        self.metrics.read().await.clone()
    }
}

fn master_stack_layout(
    window_count: usize,
    pattern: &TilingPattern,
    area: Rect,
) -> Result<Vec<Rect>> {
    if window_count == 1 {
        return Ok(vec![apply_margin(area, pattern.window_margin)?]);
    }

    let margin = pattern.window_margin as f64;
    let gap = pattern.gap_size as f64;

    let usable_width = area.size.width - 2.0 * margin;
    let usable_height = area.size.height - 2.0 * margin;
    if usable_width <= 0.0 || usable_height <= 0.0 {
        return Err(
            TilleRSError::ValidationError("Available area too small for tiling".into()).into(),
        );
    }

    let origin_x = area.origin.x + margin;
    let origin_y = area.origin.y + margin;

    let main_width = (usable_width * pattern.main_area_ratio).max(usable_width * 0.4);
    let stack_width = usable_width - main_width;

    let stack_windows = window_count - 1;
    let stack_total_gap = gap * (stack_windows.saturating_sub(1) as f64);
    let stack_height = if stack_windows > 0 {
        (usable_height - stack_total_gap) / stack_windows as f64
    } else {
        usable_height
    };

    let mut frames = Vec::with_capacity(window_count);
    frames.push(make_rect(origin_x, origin_y, main_width, usable_height)?);

    for index in 0..stack_windows {
        let y = origin_y + index as f64 * (stack_height + gap);
        frames.push(make_rect(
            origin_x + main_width + gap,
            y,
            stack_width - gap,
            stack_height,
        )?);
    }

    Ok(frames)
}

fn grid_layout(window_count: usize, pattern: &TilingPattern, area: Rect) -> Result<Vec<Rect>> {
    let margin = pattern.window_margin as f64;
    let gap = pattern.gap_size as f64;

    let usable_width = area.size.width - 2.0 * margin;
    let usable_height = area.size.height - 2.0 * margin;

    let cols = (window_count as f64).sqrt().ceil() as usize;
    let rows = (window_count as f64 / cols as f64).ceil() as usize;

    let total_gap_x = gap * (cols.saturating_sub(1) as f64);
    let total_gap_y = gap * (rows.saturating_sub(1) as f64);

    let cell_width = (usable_width - total_gap_x) / cols as f64;
    let cell_height = (usable_height - total_gap_y) / rows as f64;

    let mut frames = Vec::with_capacity(window_count);
    for index in 0..window_count {
        let row = index / cols;
        let col = index % cols;

        let x = area.origin.x + margin + col as f64 * (cell_width + gap);
        let y = area.origin.y + margin + row as f64 * (cell_height + gap);
        frames.push(make_rect(x, y, cell_width, cell_height)?);
    }

    Ok(frames)
}

fn columns_layout(window_count: usize, pattern: &TilingPattern, area: Rect) -> Result<Vec<Rect>> {
    let margin = pattern.window_margin as f64;
    let gap = pattern.gap_size as f64;

    let usable_width = area.size.width - 2.0 * margin;
    let usable_height = area.size.height - 2.0 * margin;
    let total_gap = gap * (window_count.saturating_sub(1) as f64);
    let column_width = (usable_width - total_gap) / window_count as f64;

    let mut frames = Vec::with_capacity(window_count);
    for index in 0..window_count {
        let x = area.origin.x + margin + index as f64 * (column_width + gap);
        frames.push(make_rect(
            x,
            area.origin.y + margin,
            column_width,
            usable_height,
        )?);
    }

    Ok(frames)
}

fn custom_fallback(window_count: usize, area: Rect) -> Result<Vec<Rect>> {
    // For custom patterns fall back to equal columns until the scripting DSL is implemented.
    columns_layout(
        window_count,
        &TilingPattern {
            id: uuid::Uuid::new_v4(),
            name: "Custom".into(),
            layout_algorithm: LayoutAlgorithm::Columns,
            main_area_ratio: 0.6,
            gap_size: 8,
            window_margin: 8,
            max_windows: window_count as u32,
            resize_behavior: crate::models::tiling_pattern::ResizeBehavior::Stack,
        },
        area,
    )
}

fn make_rect(x: f64, y: f64, width: f64, height: f64) -> Result<Rect> {
    let size = Size::new(width.max(1.0), height.max(1.0))?;
    Rect::new(Point::new(x, y), size)
}

fn apply_margin(area: Rect, margin: u32) -> Result<Rect> {
    let margin = margin as f64;
    make_rect(
        area.origin.x + margin,
        area.origin.y + margin,
        area.size.width - 2.0 * margin,
        area.size.height - 2.0 * margin,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tiling_pattern::{LayoutAlgorithm, ResizeBehavior, TilingPattern};

    fn sample_pattern(algorithm: LayoutAlgorithm) -> TilingPattern {
        TilingPattern {
            id: uuid::Uuid::new_v4(),
            name: "Sample".into(),
            layout_algorithm: algorithm,
            main_area_ratio: 0.6,
            gap_size: 10,
            window_margin: 10,
            max_windows: 6,
            resize_behavior: ResizeBehavior::Shrink,
        }
    }

    fn work_area() -> Rect {
        Rect::new(Point::new(0.0, 0.0), Size::new(1920.0, 1080.0).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn master_stack_allocates_main_area() {
        let engine = TilingEngine::new();
        let pattern = sample_pattern(LayoutAlgorithm::MasterStack);
        let layouts = engine
            .layout_windows(&[1, 2, 3], &pattern, work_area())
            .await
            .unwrap();

        assert_eq!(layouts.len(), 3);
        assert!(layouts[0].frame.size.width > layouts[1].frame.size.width);
    }

    #[tokio::test]
    async fn grid_layout_produces_uniform_cells() {
        let engine = TilingEngine::new();
        let pattern = sample_pattern(LayoutAlgorithm::Grid);
        let layouts = engine
            .layout_windows(&[1, 2, 3, 4], &pattern, work_area())
            .await
            .unwrap();

        assert_eq!(layouts.len(), 4);
        let first_area = layouts[0].frame.size.width * layouts[0].frame.size.height;
        let last_area =
            layouts.last().unwrap().frame.size.width * layouts.last().unwrap().frame.size.height;
        assert!((first_area - last_area).abs() < 1e-3);
    }

    #[tokio::test]
    async fn empty_window_list_returns_error() {
        let engine = TilingEngine::new();
        let pattern = sample_pattern(LayoutAlgorithm::Columns);
        let error = engine
            .layout_windows(&[], &pattern, work_area())
            .await
            .unwrap_err();

        assert!(error.to_string().contains("At least one window"));
    }
}
