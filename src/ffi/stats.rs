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

}

impl Default for CallStatistics {
    fn default() -> Self {
        Self::new()
    }
}
