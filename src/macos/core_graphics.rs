use crate::{Result, TilleRSError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Rectangle describing monitor bounds
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Result<Self> {
        if width <= 0.0 || height <= 0.0 {
            return Err(TilleRSError::ValidationError(
                "Monitor dimensions must be positive".to_string(),
            )
            .into());
        }

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
}

/// Monitors reported by Core Graphics
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub bounds: Bounds,
    pub scale_factor: f64,
    pub is_primary: bool,
}

impl MonitorInfo {
    pub fn primary(id: impl Into<String>, bounds: Bounds, scale_factor: f64) -> Self {
        Self {
            id: id.into(),
            name: "Primary".to_string(),
            bounds,
            scale_factor,
            is_primary: true,
        }
    }
}

/// Abstraction over Core Graphics display enumeration
pub trait DisplayProvider: Send + Sync {
    /// Snapshot all monitors currently available
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>>;

    /// Query a monitor by identifier
    fn get_monitor(&self, id: &str) -> Result<Option<MonitorInfo>>;
}

/// Default system-backed display provider placeholder
#[derive(Debug, Default)]
pub struct SystemDisplayProvider {
    cache: Arc<RwLock<HashMap<String, MonitorInfo>>>,
}

impl SystemDisplayProvider {
    pub fn new() -> Self {
        Self {
            cache: Arc::default(),
        }
    }
}

impl DisplayProvider for SystemDisplayProvider {
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>> {
        // The actual Core Graphics implementation will populate the cache. For now we
        // surface a friendly error so higher layers can fall back to safe defaults.
        Err(TilleRSError::MacOSAPIError(
            "SystemDisplayProvider is not implemented in this environment".into(),
        )
        .into())
    }

    fn get_monitor(&self, id: &str) -> Result<Option<MonitorInfo>> {
        Ok(self.cache.read().expect("poisoned lock").get(id).cloned())
    }
}

/// In-memory display provider for testing
#[cfg(test)]
#[derive(Debug, Default)]
pub struct InMemoryDisplayProvider {
    monitors: RwLock<HashMap<String, MonitorInfo>>,
}

#[cfg(test)]
impl InMemoryDisplayProvider {
    pub fn new_with(monitors: Vec<MonitorInfo>) -> Self {
        let mut map = HashMap::new();
        for monitor in monitors {
            map.insert(monitor.id.clone(), monitor);
        }
        Self {
            monitors: RwLock::new(map),
        }
    }
}

#[cfg(test)]
impl DisplayProvider for InMemoryDisplayProvider {
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>> {
        let mut monitors: Vec<MonitorInfo> =
            self.monitors.read().unwrap().values().cloned().collect();
        monitors.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(monitors)
    }

    fn get_monitor(&self, id: &str) -> Result<Option<MonitorInfo>> {
        Ok(self.monitors.read().unwrap().get(id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_validation_rejects_invalid_sizes() {
        let result = Bounds::new(0.0, 0.0, -1.0, 10.0);
        assert!(result.is_err());
    }

    #[test]
    fn in_memory_provider_lists_monitors_sorted() {
        let bounds = Bounds::new(0.0, 0.0, 1920.0, 1080.0).unwrap();
        let provider = InMemoryDisplayProvider::new_with(vec![
            MonitorInfo {
                id: "b".to_string(),
                name: "Secondary".to_string(),
                bounds,
                scale_factor: 2.0,
                is_primary: false,
            },
            MonitorInfo::primary("a", bounds, 2.0),
        ]);

        let monitors = provider.list_monitors().unwrap();
        assert_eq!(monitors[0].id, "a");
        assert_eq!(monitors[1].id, "b");
    }
}
