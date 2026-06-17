use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SleepCycleType {
    Daily,
    Weekly,
    Manual,
}

impl SleepCycleType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Manual => "manual",
        }
    }
}

impl std::fmt::Display for SleepCycleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SleepCycleStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl SleepCycleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for SleepCycleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_cycle_type_serializes_to_contract_values() {
        assert_eq!(SleepCycleType::Daily.to_string(), "daily");
        assert_eq!(SleepCycleType::Weekly.to_string(), "weekly");
        assert_eq!(SleepCycleType::Manual.to_string(), "manual");
    }

    #[test]
    fn sleep_cycle_status_serializes_to_contract_values() {
        assert_eq!(SleepCycleStatus::Pending.to_string(), "pending");
        assert_eq!(SleepCycleStatus::Running.to_string(), "running");
        assert_eq!(SleepCycleStatus::Completed.to_string(), "completed");
        assert_eq!(SleepCycleStatus::Failed.to_string(), "failed");
        assert_eq!(SleepCycleStatus::Cancelled.to_string(), "cancelled");
    }
}
