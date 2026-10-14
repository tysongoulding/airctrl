use airctrl_core::CoreError;
use airctrl_core::platform::PlatformWindowOps;
use airctrl_core::types::{ProcessInfo, WindowDetails};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System};

#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    pub ttl: Duration,
    pub debounce_interval: Duration,
    pub fuzzy_threshold: f32,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_millis(500),
            debounce_interval: Duration::from_millis(150),
            fuzzy_threshold: 0.65,
        }
    }
}

pub struct WindowContextMap {
    windows: Vec<WindowDetails>,
    processes: Vec<ProcessInfo>,
    active_window: Option<WindowDetails>,
    by_id: HashMap<u64, WindowDetails>,
    by_process_name: HashMap<String, Vec<u64>>,
    by_pid: HashMap<u32, Vec<u64>>,
    z_order: Vec<u64>,
    last_refreshed: Instant,
    is_dirty: AtomicBool,
    config: CrawlerConfig,
}

impl WindowContextMap {
    pub fn new(config: CrawlerConfig) -> Self {
        Self {
            windows: Vec::new(),
            processes: Vec::new(),
            active_window: None,
            by_id: HashMap::new(),
            by_process_name: HashMap::new(),
            by_pid: HashMap::new(),
            z_order: Vec::new(),
            last_refreshed: Instant::now() - Duration::from_secs(60), // Force initial refresh
            is_dirty: AtomicBool::new(true),
            config,
        }
    }

    pub fn is_fresh(&self) -> bool {
        !self.is_dirty.load(Ordering::Relaxed) && self.last_refreshed.elapsed() < self.config.ttl
    }

    pub fn should_debounce(&self) -> bool {
        self.last_refreshed.elapsed() < self.config.debounce_interval
    }

    pub fn invalidate(&self) {
        self.is_dirty.store(true, Ordering::SeqCst);
    }

    pub fn refresh(
        &mut self,
        window_ops: &dyn PlatformWindowOps,
        sys: &mut System,
    ) -> Result<(), CoreError> {
        // Enumerate windows from platform provider
        let windows = window_ops.enumerate_window_details()?;
        let active_window = window_ops.get_active_window_details().ok();

        // Refresh system processes using sysinfo
        sys.refresh_processes(ProcessesToUpdate::All, true);

        let mut processes = Vec::new();
        let mut by_pid = HashMap::new();
        let mut by_process_name = HashMap::new();
        let mut by_id = HashMap::new();

        for win in &windows {
            by_id.insert(win.id, win.clone());
            by_pid.entry(win.process_id).or_insert_with(Vec::new).push(win.id);
            if !win.process_name.is_empty() {
                by_process_name
                    .entry(win.process_name.to_lowercase())
                    .or_insert_with(Vec::new)
                    .push(win.id);
            }
        }

        for (pid, proc) in sys.processes() {
            let pid_u32 = pid.as_u32();
            let proc_name = proc.name().to_string_lossy().to_string();
            let exe_path = proc.exe().map(|p| p.to_string_lossy().to_string());
            let cpu_usage = proc.cpu_usage();
            let memory_bytes = proc.memory();
            let window_ids = by_pid.get(&pid_u32).cloned().unwrap_or_default();

            processes.push(ProcessInfo {
                pid: pid_u32,
                name: proc_name,
                exe_path,
                cpu_usage,
                memory_bytes,
                window_ids,
            });
        }

        self.z_order = windows.iter().map(|w| w.id).collect();
        self.windows = windows;
        self.processes = processes;
        self.active_window = active_window;
        self.by_id = by_id;
        self.by_pid = by_pid;
        self.by_process_name = by_process_name;
        self.last_refreshed = Instant::now();
        self.is_dirty.store(false, Ordering::SeqCst);

        Ok(())
    }

    pub fn z_order(&self) -> &[u64] {
        &self.z_order
    }

    pub fn windows(&self) -> &[WindowDetails] {
        &self.windows
    }

    pub fn processes(&self) -> &[ProcessInfo] {
        &self.processes
    }

    pub fn active_window(&self) -> Option<&WindowDetails> {
        self.active_window.as_ref()
    }

    pub fn get_window(&self, id: u64) -> Option<&WindowDetails> {
        self.by_id.get(&id)
    }

    pub fn get_windows_by_process(&self, proc_name: &str) -> Vec<&WindowDetails> {
        let key = proc_name.to_lowercase();
        self.by_process_name
            .get(&key)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }
}
