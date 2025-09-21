#![cfg(target_os = "macos")]

use std::time::Instant;

use tempfile::TempDir;
use tillers::{
    config::simple_persistence::SimplePersistenceConfig,
    macos::accessibility::{Point, Rect, Size},
    models::tiling_pattern::{LayoutAlgorithm, ResizeBehavior, TilingPattern},
    services::{
        tiling_engine::TilingEngine,
        workspace_manager::{WorkspaceManager, WorkspaceManagerConfig},
    },
    WorkspaceCreateRequest,
};
use tokio::task::yield_now;
use uuid::Uuid;

const WORKSPACES_PER_CYCLE: usize = 24;
const CYCLES: usize = 12;
const MAX_WORKSPACE_NET_GROWTH_MB: f64 = 24.0;
const MAX_WORKSPACE_PEAK_GROWTH_MB: f64 = 32.0;
const MAX_TILING_PEAK_GROWTH_MB: f64 = 12.0;
const MAX_TILING_NET_GROWTH_MB: f64 = 6.0;

#[derive(Clone)]
struct MemorySnapshot {
    rss_bytes: u64,
    virtual_bytes: u64,
    timestamp: Instant,
}

impl MemorySnapshot {
    fn current() -> Self {
        let (rss_bytes, virtual_bytes) = read_process_memory();
        Self {
            rss_bytes,
            virtual_bytes,
            timestamp: Instant::now(),
        }
    }

    fn rss_mb(&self) -> f64 {
        bytes_to_mb(self.rss_bytes)
    }
}

struct MemoryTracker {
    baseline: MemorySnapshot,
    samples: Vec<MemorySnapshot>,
}

impl MemoryTracker {
    fn start() -> Self {
        let baseline = MemorySnapshot::current();
        Self {
            baseline: baseline.clone(),
            samples: vec![baseline],
        }
    }

    fn sample(&mut self) -> MemorySnapshot {
        let mut snapshot = MemorySnapshot::current();
        if snapshot.rss_bytes == 0 && snapshot.virtual_bytes == 0 {
            if let Some(previous) = self.samples.last().cloned() {
                snapshot = previous;
            } else {
                snapshot = self.baseline.clone();
            }
        }
        self.samples.push(snapshot.clone());
        snapshot
    }

    fn net_rss_growth_mb(&self) -> f64 {
        self.samples
            .last()
            .map(|sample| bytes_to_mb(sample.rss_bytes.saturating_sub(self.baseline.rss_bytes)))
            .unwrap_or(0.0)
    }

    fn peak_rss_growth_mb(&self) -> f64 {
        self.samples
            .iter()
            .map(|sample| sample.rss_bytes.saturating_sub(self.baseline.rss_bytes))
            .max()
            .map(bytes_to_mb)
            .unwrap_or(0.0)
    }

    fn sustained_growth_mb(&self, window: usize) -> f64 {
        if window == 0 || self.samples.len() <= window {
            return 0.0;
        }
        let start = self.samples[self.samples.len() - window].rss_bytes;
        let end = self.samples.last().unwrap().rss_bytes;
        bytes_to_mb(end.saturating_sub(start))
    }

    fn samples_len(&self) -> usize {
        self.samples.len()
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn read_process_memory() -> (u64, u64) {
    use libc::{
        integer_t, kern_return_t, mach_msg_type_number_t, mach_task_basic_info, mach_task_self,
        task_info, KERN_SUCCESS, MACH_TASK_BASIC_INFO, MACH_TASK_BASIC_INFO_COUNT,
    };
    use std::mem::MaybeUninit;

    unsafe {
        let mut info = MaybeUninit::<mach_task_basic_info>::uninit();
        let mut count: mach_msg_type_number_t = MACH_TASK_BASIC_INFO_COUNT;
        let result: kern_return_t = task_info(
            mach_task_self(),
            MACH_TASK_BASIC_INFO,
            info.as_mut_ptr() as *mut integer_t,
            &mut count,
        );
        if result == KERN_SUCCESS {
            let info = info.assume_init();
            (info.resident_size as u64, info.virtual_size as u64)
        } else {
            (0, 0)
        }
    }
}

struct WorkspaceHarness {
    manager: WorkspaceManager,
    default_pattern_id: Uuid,
    _temp_dir: TempDir,
}

fn workspace_harness() -> WorkspaceHarness {
    let temp_dir = TempDir::new().expect("create temp dir");
    let persistence_config = SimplePersistenceConfig {
        config_dir: temp_dir.path().to_path_buf(),
    };
    let config = WorkspaceManagerConfig {
        max_workspaces: 512,
        auto_save: false,
        auto_save_interval: 60,
        restore_last_active: false,
        performance_monitoring: true,
    };
    let manager =
        WorkspaceManager::new_with_persistence(config, persistence_config).expect("manager");

    WorkspaceHarness {
        manager,
        default_pattern_id: Uuid::new_v4(),
        _temp_dir: temp_dir,
    }
}

fn workspace_request(index: usize) -> WorkspaceCreateRequest {
    WorkspaceCreateRequest {
        name: format!("Memory Test {}", index),
        description: Some(format!("Memory leak detection request {}", index)),
        keyboard_shortcut: shortcut_for(index),
        tiling_pattern_id: None,
        auto_arrange: Some(true),
    }
}

fn shortcut_for(index: usize) -> String {
    const SHORTCUT_KEYS: [&str; 8] = ["1", "2", "3", "4", "5", "6", "7", "8"];
    let key = SHORTCUT_KEYS[index % SHORTCUT_KEYS.len()];
    let variant = (index / SHORTCUT_KEYS.len()) % 4;

    let mut modifiers = vec!["opt"];
    if variant & 0b01 == 0b01 {
        modifiers.push("shift");
    }
    if variant & 0b10 == 0b10 {
        modifiers.push("ctrl");
    }

    modifiers.push(key);
    modifiers.join("+")
}

fn sample_pattern() -> TilingPattern {
    TilingPattern::new(
        "Memory Tiling".to_string(),
        LayoutAlgorithm::MasterStack,
        0.6,
        8,
        16,
        12,
        ResizeBehavior::Shrink,
    )
    .expect("valid tiling pattern")
}

#[tokio::test]
async fn workspace_creation_cycles_release_memory() {
    let harness = workspace_harness();
    let manager = &harness.manager;
    let mut tracker = MemoryTracker::start();

    for cycle in 0..CYCLES {
        let mut ids = Vec::with_capacity(WORKSPACES_PER_CYCLE);
        for offset in 0..WORKSPACES_PER_CYCLE {
            let request = workspace_request(cycle * WORKSPACES_PER_CYCLE + offset);
            let id = manager
                .create_workspace(request, harness.default_pattern_id)
                .await
                .expect("create workspace");
            ids.push(id);
        }

        tracker.sample();

        for id in ids {
            manager
                .delete_workspace(id)
                .await
                .expect("delete workspace");
        }

        yield_now().await;
        tracker.sample();
    }

    yield_now().await;
    tracker.sample();

    let net = tracker.net_rss_growth_mb();
    let peak = tracker.peak_rss_growth_mb();
    let sustained = tracker.sustained_growth_mb(6);

    println!(
        "Workspace cycles -> net: {:.2} MB, peak: {:.2} MB, sustained(6): {:.2} MB, samples: {}",
        net,
        peak,
        sustained,
        tracker.samples_len()
    );

    assert!(
        net < MAX_WORKSPACE_NET_GROWTH_MB,
        "Workspace cycles leaked memory: net {:.2} MB (peak {:.2} MB)",
        net,
        peak
    );
    assert!(
        peak < MAX_WORKSPACE_PEAK_GROWTH_MB,
        "Workspace cycles peaked above threshold: {:.2} MB",
        peak
    );
    assert!(
        sustained < 8.0,
        "Workspace cycles show sustained RSS growth over last samples: {:.2} MB",
        sustained
    );
}

#[tokio::test]
async fn sustained_workspace_activity_remains_bounded() {
    let harness = workspace_harness();
    let manager = &harness.manager;
    let mut tracker = MemoryTracker::start();

    let mut workspace_ids = Vec::with_capacity(WORKSPACES_PER_CYCLE);
    for index in 0..WORKSPACES_PER_CYCLE {
        let id = manager
            .create_workspace(workspace_request(index), harness.default_pattern_id)
            .await
            .expect("create workspace");
        workspace_ids.push(id);
    }

    tracker.sample();

    for round in 0..160usize {
        let id = workspace_ids[round % workspace_ids.len()];
        manager
            .switch_to_workspace(id)
            .await
            .expect("switch workspace");
        if round % 20 == 0 {
            tracker.sample();
        }
    }

    for id in workspace_ids {
        manager
            .delete_workspace(id)
            .await
            .expect("delete workspace");
    }

    yield_now().await;
    tracker.sample();

    let net = tracker.net_rss_growth_mb();
    let peak = tracker.peak_rss_growth_mb();
    let sustained = tracker.sustained_growth_mb(5);

    println!(
        "Sustained activity -> net: {:.2} MB, peak: {:.2} MB, sustained(5): {:.2} MB",
        net, peak, sustained
    );

    assert!(
        net < MAX_WORKSPACE_NET_GROWTH_MB,
        "Sustained workspace activity leaked memory: net {:.2} MB (peak {:.2} MB)",
        net,
        peak
    );
    assert!(
        sustained < 6.0,
        "Sustained workspace activity retained {:.2} MB across last samples",
        sustained
    );
}

#[tokio::test]
async fn tiling_engine_layouts_stay_bounded() {
    let mut tracker = MemoryTracker::start();
    let engine = TilingEngine::new();
    let pattern = sample_pattern();
    let area = Rect {
        origin: Point { x: 0.0, y: 0.0 },
        size: Size {
            width: 1920.0,
            height: 1080.0,
        },
    };

    for iteration in 0..180u32 {
        let window_count = (iteration % 12) + 1;
        let window_ids: Vec<u32> = (1..=window_count).collect();
        engine
            .layout_windows(&window_ids, &pattern, area.clone())
            .await
            .expect("layout windows");

        if iteration % 30 == 0 {
            tracker.sample();
        }
    }

    tracker.sample();

    let net = tracker.net_rss_growth_mb();
    let peak = tracker.peak_rss_growth_mb();

    println!("Tiling engine -> net: {:.2} MB, peak: {:.2} MB", net, peak);

    assert!(
        peak < MAX_TILING_PEAK_GROWTH_MB,
        "Tiling engine peak RSS {:.2} MB exceeded threshold (net {:.2} MB)",
        peak,
        net
    );
    assert!(
        net < MAX_TILING_NET_GROWTH_MB,
        "Tiling engine net RSS {:.2} MB exceeded threshold (peak {:.2} MB)",
        net,
        peak
    );
}
