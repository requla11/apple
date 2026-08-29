use crate::protocol::{ExecutionResult, ViolationRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub task_id: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub peak_memory_bytes: u64,
    pub hermetic_guarantee: bool,
    pub total_violations: usize,
    pub violations: Vec<ViolationRecord>,
}

#[derive(Debug, Default, Clone)]
pub struct AuditStore {
    records: Arc<Mutex<HashMap<String, AuditReport>>>,
}

impl AuditStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_result(&self, result: &ExecutionResult) {
        let report = AuditReport {
            task_id: result.task_id.clone(),
            exit_code: result.exit_code,
            duration_ms: result.execution_duration_ms,
            peak_memory_bytes: result.peak_memory_bytes,
            hermetic_guarantee: result.hermetic_guarantee,
            total_violations: result.violations.len(),
            violations: result.violations.clone(),
        };
        if let Ok(mut map) = self.records.lock() {
            map.insert(result.task_id.clone(), report);
        }
    }

    pub fn get_report(&self, task_id: &str) -> Option<AuditReport> {
        self.records.lock().ok()?.get(task_id).cloned()
    }

    pub fn list_task_ids(&self) -> Vec<String> {
        self.records
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}
