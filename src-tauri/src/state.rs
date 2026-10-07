//! 全局状态：配置、正在运行的任务、日志序号

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::model::AppConfig;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub config_path: Mutex<Option<PathBuf>>,
    /// task_id -> pid
    pub running: Mutex<HashMap<String, u32>>,
    log_seq: AtomicU64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Mutex::new(AppConfig::default()),
            config_path: Mutex::new(None),
            running: Mutex::new(HashMap::new()),
            log_seq: AtomicU64::new(1),
        }
    }

    pub fn next_seq(&self) -> u64 {
        self.log_seq.fetch_add(1, Ordering::Relaxed)
    }

    pub fn config_snapshot(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn update_config<F: FnOnce(&mut AppConfig)>(&self, f: F) -> AppConfig {
        let mut guard = self.config.lock().unwrap();
        f(&mut guard);
        let snapshot = guard.clone();
        drop(guard);
        snapshot
    }

    pub fn register_task(&self, task: &str, pid: u32) {
        self.running.lock().unwrap().insert(task.to_string(), pid);
    }

    pub fn clear_task(&self, task: &str) {
        self.running.lock().unwrap().remove(task);
    }

    pub fn task_pid(&self, task: &str) -> Option<u32> {
        self.running.lock().unwrap().get(task).copied()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
