use crate::macos::accessibility::{
    AXWindow, AccessibilityProvider, InMemoryAccessibilityProvider, Point, Rect, Size,
};
use crate::macos::core_graphics::{DisplayProvider, InMemoryDisplayProvider, MonitorInfo};
use crate::{Result, TilleRSError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// High-level representation of a managed window
#[derive(Debug, Clone, PartialEq)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub application_name: String,
    pub bundle_id: String,
    pub position: Point,
    pub size: Size,
    pub is_minimized: bool,
    pub is_fullscreen: bool,
    pub is_focused: bool,
    pub monitor_id: Option<String>,
    pub mode: WindowMode,
}

impl WindowInfo {
    fn from_ax(ax: &AXWindow, mode: WindowMode) -> Self {
        Self {
            id: ax.window_id,
            title: ax.title.clone(),
            application_name: ax.application_name.clone(),
            bundle_id: ax.bundle_id.clone(),
            position: ax.frame.origin,
            size: ax.frame.size,
            is_minimized: ax.is_minimized,
            is_fullscreen: ax.is_fullscreen,
            is_focused: ax.is_focused,
            monitor_id: ax.monitor_id.clone(),
            mode,
        }
    }

    pub fn frame(&self) -> Rect {
        Rect {
            origin: self.position,
            size: self.size,
        }
    }
}

/// Window behaviour managed by the service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Tiled,
    Floating,
    Fullscreen,
}

/// Internal state for cached windows
#[derive(Debug, Clone)]
struct ManagedWindow {
    info: WindowInfo,
}

/// Telemetry for window operations
#[derive(Debug, Default, Clone)]
pub struct WindowManagerMetrics {
    pub refresh_count: u64,
    pub focus_events: u64,
    pub move_events: u64,
    pub minimize_events: u64,
    pub error_count: u64,
}

/// Service responsible for high level window manipulation
pub struct WindowManager {
    accessibility: Arc<dyn AccessibilityProvider>,
    displays: Arc<dyn DisplayProvider>,
    cache: Arc<RwLock<HashMap<u32, ManagedWindow>>>,
    metrics: Arc<RwLock<WindowManagerMetrics>>,
}

impl WindowManager {
    /// Construct a window manager backed by the in-memory providers. This is
    /// suitable for tests and for the current stubbed binary entry point.
    pub fn with_default_providers() -> Self {
        let accessibility: Arc<dyn AccessibilityProvider> =
            Arc::new(InMemoryAccessibilityProvider::default());
        let displays: Arc<dyn DisplayProvider> = Arc::new(InMemoryDisplayProvider::default());
        Self::new(accessibility, displays)
    }

    pub fn new(
        accessibility: Arc<dyn AccessibilityProvider>,
        displays: Arc<dyn DisplayProvider>,
    ) -> Self {
        Self {
            accessibility,
            displays,
            cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(WindowManagerMetrics::default())),
        }
    }

    /// Ensure the service has permission to manage windows
    pub async fn ensure_permissions(&self) -> Result<()> {
        match self.accessibility.ensure_permissions() {
            Ok(_) => Ok(()),
            Err(err) => {
                self.increment_error().await;
                Err(err)
            }
        }
    }

    /// Refresh the internal window cache from the accessibility provider
    pub async fn refresh_cache(&self) -> Result<()> {
        debug!("Refreshing window cache");
        let windows = match self.accessibility.list_windows(true) {
            Ok(result) => result,
            Err(err) => {
                self.increment_error().await;
                return Err(err);
            }
        };

        let monitors = self.displays.list_monitors().ok();

        let mut cache = self.cache.write().await;
        cache.clear();

        for window in windows {
            let mode = if window.is_fullscreen {
                WindowMode::Fullscreen
            } else {
                WindowMode::Tiled
            };

            let mut info = WindowInfo::from_ax(&window, mode);
            if info.monitor_id.is_none() {
                info.monitor_id = infer_monitor(&info, monitors.as_deref());
            }

            cache.insert(window.window_id, ManagedWindow { info });
        }

        self.metrics.write().await.refresh_count += 1;
        Ok(())
    }

    /// List windows, optionally filtering by monitor and minimized state
    pub async fn list_windows(
        &self,
        include_minimized: bool,
        monitor_filter: Option<&str>,
    ) -> Result<Vec<WindowInfo>> {
        {
            let cache = self.cache.read().await;
            if cache.is_empty() {
                drop(cache);
                self.refresh_cache().await?;
            }
        }

        let cache = self.cache.read().await;
        let mut windows: Vec<_> = cache
            .values()
            .filter(|entry| include_minimized || !entry.info.is_minimized)
            .filter(|entry| match monitor_filter {
                Some(monitor) => entry.info.monitor_id.as_deref() == Some(monitor),
                None => true,
            })
            .map(|entry| entry.info.clone())
            .collect();

        windows.sort_by_key(|window| window.id);
        Ok(windows)
    }

    /// Retrieve a single window
    pub async fn get_window(&self, window_id: u32) -> Result<Option<WindowInfo>> {
        {
            let cache = self.cache.read().await;
            if let Some(window) = cache.get(&window_id) {
                return Ok(Some(window.info.clone()));
            }
        }

        self.refresh_cache().await?;
        let cache = self.cache.read().await;
        Ok(cache.get(&window_id).map(|entry| entry.info.clone()))
    }

    /// Focus a window via the accessibility provider
    pub async fn focus_window(&self, window_id: u32) -> Result<()> {
        if let Err(err) = self.accessibility.focus_window(window_id) {
            self.increment_error().await;
            return Err(err);
        }

        let mut cache = self.cache.write().await;
        for entry in cache.values_mut() {
            entry.info.is_focused = entry.info.id == window_id;
        }

        self.metrics.write().await.focus_events += 1;
        Ok(())
    }

    /// Move / resize a window and update cached state
    pub async fn set_window_frame(&self, window_id: u32, frame: Rect, animate: bool) -> Result<()> {
        if let Err(err) = self
            .accessibility
            .set_window_frame(window_id, frame, animate)
        {
            self.increment_error().await;
            return Err(err);
        }

        {
            let mut cache = self.cache.write().await;
            if let Some(window) = cache.get_mut(&window_id) {
                window.info.position = frame.origin;
                window.info.size = frame.size;
            } else {
                drop(cache);
                self.refresh_cache().await?;
            }
        }

        self.metrics.write().await.move_events += 1;
        Ok(())
    }

    /// Minimize or restore a window
    pub async fn set_window_minimized(&self, window_id: u32, minimized: bool) -> Result<()> {
        if let Err(err) = self
            .accessibility
            .set_window_minimized(window_id, minimized)
        {
            self.increment_error().await;
            return Err(err);
        }

        let mut cache = self.cache.write().await;
        match cache.get_mut(&window_id) {
            Some(window) => {
                window.info.is_minimized = minimized;
            }
            None => {
                drop(cache);
                self.refresh_cache().await?;
            }
        }

        self.metrics.write().await.minimize_events += 1;
        Ok(())
    }

    /// Toggle floating mode for a window
    pub async fn toggle_floating(&self, window_id: u32) -> Result<WindowMode> {
        let mut cache = self.cache.write().await;
        if let Some(window) = cache.get_mut(&window_id) {
            if window.info.mode == WindowMode::Fullscreen {
                return Ok(WindowMode::Fullscreen);
            }

            window.info.mode = match window.info.mode {
                WindowMode::Tiled => WindowMode::Floating,
                WindowMode::Floating => WindowMode::Tiled,
                WindowMode::Fullscreen => WindowMode::Fullscreen,
            };

            return Ok(window.info.mode);
        }

        drop(cache);
        self.refresh_cache().await?;
        let mut cache = self.cache.write().await;
        match cache.get_mut(&window_id) {
            Some(window) => {
                if window.info.mode == WindowMode::Fullscreen {
                    Ok(WindowMode::Fullscreen)
                } else {
                    window.info.mode = WindowMode::Floating;
                    Ok(window.info.mode)
                }
            }
            None => {
                self.increment_error().await;
                Err(TilleRSError::WindowNotFound(window_id).into())
            }
        }
    }

    pub async fn metrics(&self) -> WindowManagerMetrics {
        self.metrics.read().await.clone()
    }

    async fn increment_error(&self) {
        self.metrics.write().await.error_count += 1;
    }
}

fn infer_monitor(window: &WindowInfo, monitors: Option<&[MonitorInfo]>) -> Option<String> {
    let monitors = monitors?;
    monitors
        .iter()
        .find(|monitor| {
            let within_x = window.position.x >= monitor.bounds.x
                && window.position.x <= monitor.bounds.x + monitor.bounds.width;
            let within_y = window.position.y >= monitor.bounds.y
                && window.position.y <= monitor.bounds.y + monitor.bounds.height;
            within_x && within_y
        })
        .map(|monitor| monitor.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macos::accessibility::InMemoryAccessibilityProvider;
    use crate::macos::core_graphics::{Bounds, InMemoryDisplayProvider};

    fn seed_providers() -> (Arc<dyn AccessibilityProvider>, Arc<dyn DisplayProvider>) {
        let window = AXWindow::new(
            1,
            "Editor",
            "Code",
            "com.example.code",
            Rect::new(Point::new(100.0, 100.0), Size::new(1200.0, 800.0).unwrap()).unwrap(),
            false,
            false,
            true,
            Some("PRIMARY".to_string()),
        );
        let accessibility: Arc<dyn AccessibilityProvider> =
            Arc::new(InMemoryAccessibilityProvider::new_with(vec![window]));
        let monitor_bounds = Bounds::new(0.0, 0.0, 2560.0, 1440.0).unwrap();
        let display: Arc<dyn DisplayProvider> =
            Arc::new(InMemoryDisplayProvider::new_with(vec![MonitorInfo {
                id: "PRIMARY".to_string(),
                name: "Main".to_string(),
                bounds: monitor_bounds,
                scale_factor: 2.0,
                is_primary: true,
            }]));

        (accessibility, display)
    }

    #[tokio::test]
    async fn list_windows_returns_cached_entries() {
        let (accessibility, displays) = seed_providers();
        let manager = WindowManager::new(accessibility, displays);

        let windows = manager.list_windows(false, None).await.unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].title, "Editor");
    }

    #[tokio::test]
    async fn focus_window_updates_cache() {
        let (accessibility, displays) = seed_providers();
        let manager = WindowManager::new(accessibility, displays);

        manager.focus_window(1).await.unwrap();
        let window = manager.get_window(1).await.unwrap().unwrap();
        assert!(window.is_focused);

        let metrics = manager.metrics().await;
        assert_eq!(metrics.focus_events, 1);
    }

    #[tokio::test]
    async fn toggle_floating_switches_mode() {
        let (accessibility, displays) = seed_providers();
        let manager = WindowManager::new(accessibility, displays);

        // Populate cache
        manager.list_windows(false, None).await.unwrap();
        let mode = manager.toggle_floating(1).await.unwrap();
        assert_eq!(mode, WindowMode::Floating);

        let mode = manager.toggle_floating(1).await.unwrap();
        assert_eq!(mode, WindowMode::Tiled);
    }

    #[tokio::test]
    async fn set_window_frame_updates_position() {
        let (accessibility, displays) = seed_providers();
        let manager = WindowManager::new(accessibility, displays);

        manager.list_windows(false, None).await.unwrap();

        let new_frame =
            Rect::new(Point::new(200.0, 300.0), Size::new(800.0, 600.0).unwrap()).unwrap();
        manager.set_window_frame(1, new_frame, false).await.unwrap();

        let window = manager.get_window(1).await.unwrap().unwrap();
        assert_eq!(window.position.x, 200.0);
        assert_eq!(window.size.width, 800.0);
    }

    #[tokio::test]
    async fn set_window_minimized_updates_state() {
        let (accessibility, displays) = seed_providers();
        let manager = WindowManager::new(accessibility, displays);

        manager.list_windows(true, None).await.unwrap();
        manager.set_window_minimized(1, true).await.unwrap();

        let window = manager.get_window(1).await.unwrap().unwrap();
        assert!(window.is_minimized);
    }
}
