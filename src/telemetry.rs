use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResourceMetrics {
    pub task_id: String,
    pub cpu_time_ms: u64,
    pub peak_memory_mb: u64,
    pub exit_code: i32,
    pub is_clean: bool,
}

#[derive(Debug, Default, Clone)]
pub struct TelemetryCollector {
    records: Arc<Mutex<HashMap<String, ProcessResourceMetrics>>>,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_metrics(
        &self,
        task_id: impl Into<String>,
        cpu_time_ms: u64,
        peak_memory_mb: u64,
        exit_code: i32,
    ) {
        let id = task_id.into();
        let record = ProcessResourceMetrics {
            task_id: id.clone(),
            cpu_time_ms,
            peak_memory_mb,
            exit_code,
            is_clean: exit_code == 0,
        };
        if let Ok(mut map) = self.records.lock() {
            map.insert(id, record);
        }
    }

    pub fn get_metrics(&self, task_id: &str) -> Option<ProcessResourceMetrics> {
        self.records.lock().ok()?.get(task_id).cloned()
    }
}
