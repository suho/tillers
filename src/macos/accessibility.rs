use crate::{Result, TilleRSError};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// Two-dimensional point used for window positioning
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Window size in display points
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Result<Self> {
        if width <= 0.0 || height <= 0.0 {
            return Err(TilleRSError::ValidationError(
                "Window dimensions must be positive".to_string(),
            )
            .into());
        }

        Ok(Self { width, height })
    }
}

/// Convenience rectangle describing a window frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(origin: Point, size: Size) -> Result<Self> {
        Ok(Self { origin, size })
    }
}

/// Accessiblity-derived window metadata used by higher level services
#[derive(Debug, Clone, PartialEq)]
pub struct AXWindow {
    pub window_id: u32,
    pub title: String,
    pub application_name: String,
    pub bundle_id: String,
    pub frame: Rect,
    pub is_minimized: bool,
    pub is_fullscreen: bool,
    pub is_focused: bool,
    pub monitor_id: Option<String>,
}

impl AXWindow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window_id: u32,
        title: impl Into<String>,
        application_name: impl Into<String>,
        bundle_id: impl Into<String>,
        frame: Rect,
        is_minimized: bool,
        is_fullscreen: bool,
        is_focused: bool,
        monitor_id: Option<String>,
    ) -> Self {
        Self {
            window_id,
            title: title.into(),
            application_name: application_name.into(),
            bundle_id: bundle_id.into(),
            frame,
            is_minimized,
            is_fullscreen,
            is_focused,
            monitor_id,
        }
    }
}

/// Tracks accessibility permission state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    Unknown,
    Granted,
    Denied,
}

/// Abstraction for interacting with macOS Accessibility APIs
pub trait AccessibilityProvider: Send + Sync {
    /// Ensure accessibility permissions are granted
    fn ensure_permissions(&self) -> Result<()>;

    /// Retrieve permission status without prompting the user
    fn permission_status(&self) -> PermissionStatus;

    /// Snapshot all available windows
    fn list_windows(&self, include_minimized: bool) -> Result<Vec<AXWindow>>;

    /// Retrieve a single window by ID
    fn get_window(&self, window_id: u32) -> Result<Option<AXWindow>>;

    /// Focus the specified window
    fn focus_window(&self, window_id: u32) -> Result<()>;

    /// Move / resize a window to the requested frame
    fn set_window_frame(&self, window_id: u32, frame: Rect, animate: bool) -> Result<()>;

    /// Minimize or restore a window
    fn set_window_minimized(&self, window_id: u32, minimized: bool) -> Result<()>;
}

/// Default system-backed provider placeholder
#[derive(Debug)]
pub struct SystemAccessibilityProvider {
    status: Arc<RwLock<PermissionStatus>>, // shared so tests can simulate state
}

impl SystemAccessibilityProvider {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(PermissionStatus::Unknown)),
        }
    }

    fn ensure_trusted(&self) -> Result<()> {
        let status = *self.status.read().expect("poisoned lock");
        match status {
            PermissionStatus::Granted => Ok(()),
            PermissionStatus::Unknown | PermissionStatus::Denied => {
                Err(TilleRSError::PermissionDenied(
                    "Accessibility permission is required for window management".into(),
                )
                .into())
            }
        }
    }
}

impl Default for SystemAccessibilityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityProvider for SystemAccessibilityProvider {
    fn ensure_permissions(&self) -> Result<()> {
        self.ensure_trusted()
    }

    fn permission_status(&self) -> PermissionStatus {
        *self.status.read().expect("poisoned lock")
    }

    fn list_windows(&self, _include_minimized: bool) -> Result<Vec<AXWindow>> {
        Err(TilleRSError::MacOSAPIError(
            "SystemAccessibilityProvider is not implemented in this environment".into(),
        )
        .into())
    }

    fn get_window(&self, _window_id: u32) -> Result<Option<AXWindow>> {
        Err(TilleRSError::MacOSAPIError(
            "SystemAccessibilityProvider is not implemented in this environment".into(),
        )
        .into())
    }

    fn focus_window(&self, _window_id: u32) -> Result<()> {
        Err(TilleRSError::MacOSAPIError(
            "SystemAccessibilityProvider is not implemented in this environment".into(),
        )
        .into())
    }

    fn set_window_frame(&self, _window_id: u32, _frame: Rect, _animate: bool) -> Result<()> {
        Err(TilleRSError::MacOSAPIError(
            "SystemAccessibilityProvider is not implemented in this environment".into(),
        )
        .into())
    }

    fn set_window_minimized(&self, _window_id: u32, _minimized: bool) -> Result<()> {
        Err(TilleRSError::MacOSAPIError(
            "SystemAccessibilityProvider is not implemented in this environment".into(),
        )
        .into())
    }
}

/// Simple in-memory provider used for testing the higher level services
#[derive(Debug)]
pub struct InMemoryAccessibilityProvider {
    windows: RwLock<HashMap<u32, AXWindow>>,
    status: RwLock<PermissionStatus>,
}

impl InMemoryAccessibilityProvider {
    pub fn new_with(windows: Vec<AXWindow>) -> Self {
        let mut map = HashMap::new();
        for window in windows {
            map.insert(window.window_id, window);
        }

        Self {
            windows: RwLock::new(map),
            status: RwLock::new(PermissionStatus::Granted),
        }
    }

    pub fn set_permission_status(&self, status: PermissionStatus) {
        *self.status.write().unwrap() = status;
    }

    fn clear_focus(entries: &mut HashMap<u32, AXWindow>) {
        for window in entries.values_mut() {
            window.is_focused = false;
        }
    }
}

impl AccessibilityProvider for InMemoryAccessibilityProvider {
    fn ensure_permissions(&self) -> Result<()> {
        match *self.status.read().unwrap() {
            PermissionStatus::Granted => Ok(()),
            PermissionStatus::Unknown | PermissionStatus::Denied => {
                Err(TilleRSError::PermissionDenied(
                    "Accessibility permission denied in in-memory provider".to_string(),
                )
                .into())
            }
        }
    }

    fn permission_status(&self) -> PermissionStatus {
        *self.status.read().unwrap()
    }

    fn list_windows(&self, include_minimized: bool) -> Result<Vec<AXWindow>> {
        let windows = self.windows.read().unwrap();
        let mut result: Vec<AXWindow> = windows
            .values()
            .filter(|window| include_minimized || !window.is_minimized)
            .cloned()
            .collect();
        result.sort_by_key(|w| w.window_id);
        Ok(result)
    }

    fn get_window(&self, window_id: u32) -> Result<Option<AXWindow>> {
        Ok(self.windows.read().unwrap().get(&window_id).cloned())
    }

    fn focus_window(&self, window_id: u32) -> Result<()> {
        let mut windows = self.windows.write().unwrap();
        if windows.contains_key(&window_id) {
            Self::clear_focus(&mut windows);
            if let Some(window) = windows.get_mut(&window_id) {
                window.is_focused = true;
            }
            Ok(())
        } else {
            Err(TilleRSError::WindowNotFound(window_id).into())
        }
    }

    fn set_window_frame(&self, window_id: u32, frame: Rect, _animate: bool) -> Result<()> {
        let mut windows = self.windows.write().unwrap();
        if let Some(window) = windows.get_mut(&window_id) {
            window.frame = frame;
            Ok(())
        } else {
            Err(TilleRSError::WindowNotFound(window_id).into())
        }
    }

    fn set_window_minimized(&self, window_id: u32, minimized: bool) -> Result<()> {
        let mut windows = self.windows.write().unwrap();
        if let Some(window) = windows.get_mut(&window_id) {
            window.is_minimized = minimized;
            Ok(())
        } else {
            Err(TilleRSError::WindowNotFound(window_id).into())
        }
    }
}

impl Default for InMemoryAccessibilityProvider {
    fn default() -> Self {
        Self {
            windows: RwLock::new(HashMap::new()),
            status: RwLock::new(PermissionStatus::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_window(window_id: u32, minimized: bool) -> AXWindow {
        let size = Size::new(1280.0, 720.0).unwrap();
        AXWindow::new(
            window_id,
            format!("Window {window_id}"),
            "App",
            "com.example.app",
            Rect::new(Point::new(0.0, 0.0), size).unwrap(),
            minimized,
            false,
            false,
            Some("MOCK".to_string()),
        )
    }

    #[test]
    fn in_memory_provider_list_filters_minimized() {
        let provider = InMemoryAccessibilityProvider::new_with(vec![
            sample_window(1, false),
            sample_window(2, true),
        ]);

        let visible = provider.list_windows(false).unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].window_id, 1);

        let all = provider.list_windows(true).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn in_memory_provider_focus_updates_state() {
        let provider = InMemoryAccessibilityProvider::new_with(vec![
            sample_window(1, false),
            sample_window(2, false),
        ]);

        provider.focus_window(2).unwrap();

        let win1 = provider.get_window(1).unwrap().unwrap();
        let win2 = provider.get_window(2).unwrap().unwrap();
        assert!(!win1.is_focused);
        assert!(win2.is_focused);
    }

    #[test]
    fn in_memory_provider_respects_permission_status() {
        let provider = InMemoryAccessibilityProvider::default();
        provider.set_permission_status(PermissionStatus::Denied);

        let result = provider.ensure_permissions();
        assert!(result.is_err());
    }
}
