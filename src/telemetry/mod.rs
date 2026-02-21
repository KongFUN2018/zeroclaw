//! Telemetry system for tracking usage patterns
//!
//! Collects and stores local-only usage metrics for skills, tools, and MCPs.
//! No data is transmitted to the cloud.

pub mod builtin;
pub mod collector;
pub mod storage;

pub use builtin::{BuiltinCandidate, BuiltinRecommender, Recommendation, RecommendationThresholds};
pub use collector::TelemetryCollector;
pub use storage::{TelemetryEvent, TelemetryStorage};

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable/disable telemetry collection
    pub enabled: bool,
    /// Database path
    pub db_path: std::path::PathBuf,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            db_path: std::path::PathBuf::from("telemetry.db"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TelemetryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.db_path, std::path::PathBuf::from("telemetry.db"));
    }
}
