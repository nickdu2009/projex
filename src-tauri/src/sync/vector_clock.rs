//! Vector Clock implementation for conflict detection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    pub clocks: HashMap<String, i64>,
}

impl VectorClock {
    /// Create a new vector clock for a device
    pub fn new(device_id: String) -> Self {
        let mut clocks = HashMap::new();
        clocks.insert(device_id, 0);
        Self { clocks }
    }

    /// Create an empty vector clock
    pub fn empty() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment the clock for a device
    pub fn increment(&mut self, device_id: &str) {
        *self.clocks.entry(device_id.to_string()).or_insert(0) += 1;
    }

    /// Merge another vector clock into this one (take maximum)
    pub fn merge(&mut self, other: &VectorClock) {
        for (device, clock) in &other.clocks {
            let entry = self.clocks.entry(device.clone()).or_insert(0);
            *entry = (*entry).max(*clock);
        }
    }

    /// Check if this clock happened before another (causality)
    pub fn happened_before(&self, other: &VectorClock) -> bool {
        // All clocks in self must be <= clocks in other
        // At least one clock in self must be < clock in other

        let mut all_less_or_equal = true;
        let mut at_least_one_less = false;

        // Check all devices in self
        for (device, self_clock) in &self.clocks {
            let other_clock = other.clocks.get(device).unwrap_or(&0);

            if self_clock > other_clock {
                all_less_or_equal = false;
                break;
            }

            if self_clock < other_clock {
                at_least_one_less = true;
            }
        }

        // Check devices only in other
        for device in other.clocks.keys() {
            if !self.clocks.contains_key(device) {
                at_least_one_less = true;
            }
        }

        all_less_or_equal && at_least_one_less
    }

    /// Check if two clocks are concurrent (conflicting)
    pub fn conflicts_with(&self, other: &VectorClock) -> bool {
        let mut self_greater = false;
        let mut other_greater = false;

        // Collect all devices from both clocks
        let all_devices: std::collections::HashSet<_> =
            self.clocks.keys().chain(other.clocks.keys()).collect();

        for device in all_devices {
            let self_clock = self.clocks.get(device).unwrap_or(&0);
            let other_clock = other.clocks.get(device).unwrap_or(&0);

            if self_clock > other_clock {
                self_greater = true;
            } else if other_clock > self_clock {
                other_greater = true;
            }
        }

        // Concurrent if both have updates the other doesn't know about
        self_greater && other_greater
    }

    /// Get the sum of all clocks (for LWW comparison)
    pub fn sum(&self) -> i64 {
        self.clocks.values().sum()
    }

    /// Compare two clocks for LWW resolution
    /// Returns: Ordering (Less, Equal, Greater)
    pub fn compare_for_lww(
        &self,
        other: &VectorClock,
        self_device: &str,
        other_device: &str,
    ) -> std::cmp::Ordering {
        let self_sum = self.sum();
        let other_sum = other.sum();

        match self_sum.cmp(&other_sum) {
            std::cmp::Ordering::Equal => {
                // Same sum, use device_id lexicographic order
                self_device.cmp(other_device)
            }
            ordering => ordering,
        }
    }
}
