// FFI call statistics

use std::collections::HashMap;

/// Statistics for FFI function calls
#[derive(Debug, Clone)]
pub struct CallStatistics {
    calls: HashMap<String, (usize, usize)>, // (success_count, failure_count)
}

impl CallStatistics {
    pub fn new() -> Self {
        Self {
            calls: HashMap::new(),
        }
    }

    pub fn record_call(&mut self, function: &str, success: bool) {
        let entry = self.calls.entry(function.to_string()).or_insert((0, 0));
        if success {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }

    pub fn get_success_count(&self, function: &str) -> usize {
        self.calls
            .get(function)
            .map(|(success, _)| *success)
            .unwrap_or(0)
    }

    pub fn get_failure_count(&self, function: &str) -> usize {
        self.calls
            .get(function)
            .map(|(_, failure)| *failure)
            .unwrap_or(0)
    }

    pub fn get_total_calls(&self, function: &str) -> usize {
        self.calls
            .get(function)
            .map(|(success, failure)| success + failure)
            .unwrap_or(0)
    }
}

impl Default for CallStatistics {
    fn default() -> Self {
        Self::new()
    }
}
