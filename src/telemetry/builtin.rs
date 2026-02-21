//! Smart builtin recommendation engine
//!
//! Analyzes telemetry data to recommend frequently-used external skills/tools
//! for integration into the ZeroClaw core.

use crate::telemetry::storage::{ComponentStats, EventType, TelemetryStorage};
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Thresholds for builtin recommendations
#[derive(Debug, Clone)]
pub struct RecommendationThresholds {
    /// Minimum calls per 30 days
    pub min_calls_per_30d: i64,
    /// Minimum success rate (0.0 to 1.0)
    pub min_success_rate: f64,
    /// Maximum acceptable latency (ms)
    pub max_latency_ms: f64,
}

impl Default for RecommendationThresholds {
    fn default() -> Self {
        Self {
            min_calls_per_30d: 100,
            min_success_rate: 0.95,
            max_latency_ms: 100.0,
        }
    }
}

/// A candidate for builtin integration
#[derive(Debug, Clone)]
pub struct BuiltinCandidate {
    /// Component name (skill/tool/mcp)
    pub name: String,
    /// Component type
    pub component_type: String,
    /// Usage statistics
    pub stats: ComponentStats,
    /// Whether it meets all thresholds
    pub recommended: bool,
    /// Reason for recommendation or rejection
    pub reason: String,
}

/// Recommendation result
#[derive(Debug, Clone)]
pub struct Recommendation {
    /// Candidates that should be integrated
    pub recommended: Vec<BuiltinCandidate>,
    /// Candidates that don't meet thresholds
    pub not_recommended: Vec<BuiltinCandidate>,
    /// Timestamp of analysis
    pub generated_at: DateTime<Utc>,
    /// Thresholds used
    pub thresholds: RecommendationThresholds,
}

/// Builtin recommender
///
/// Analyzes telemetry data to find frequently-used components that should
/// be integrated into ZeroClaw core.
pub struct BuiltinRecommender {
    storage: TelemetryStorage,
    thresholds: RecommendationThresholds,
}

impl BuiltinRecommender {
    /// Create a new recommender
    pub fn new(storage: TelemetryStorage) -> Self {
        Self {
            storage,
            thresholds: RecommendationThresholds::default(),
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(
        storage: TelemetryStorage,
        thresholds: RecommendationThresholds,
    ) -> Self {
        Self {
            storage,
            thresholds,
        }
    }

    /// Get the current thresholds
    pub fn thresholds(&self) -> &RecommendationThresholds {
        &self.thresholds
    }

    /// Update thresholds
    pub fn set_thresholds(&mut self, thresholds: RecommendationThresholds) {
        self.thresholds = thresholds;
    }

    /// Check if a component meets recommendation thresholds
    pub fn should_recommend(&self, stats: &ComponentStats) -> bool {
        let success_rate = stats.success_rate();

        stats.total_calls >= self.thresholds.min_calls_per_30d
            && success_rate >= self.thresholds.min_success_rate
            && stats.avg_duration_ms <= self.thresholds.max_latency_ms
    }

    /// Analyze skills and generate recommendations
    pub fn recommend_skills(&self) -> Result<Recommendation> {
        let skill_names = self.storage.get_component_names(EventType::SkillUsage)?;
        let mut candidates = Vec::new();

        for name in skill_names {
            if let Ok(stats) = self
                .storage
                .get_component_stats(&name, EventType::SkillUsage)
            {
                let recommended = self.should_recommend(&stats);
                let reason = self.explain_recommendation(&stats, recommended);

                candidates.push(BuiltinCandidate {
                    name,
                    component_type: "skill".to_string(),
                    stats,
                    recommended,
                    reason,
                });
            }
        }

        Ok(self.build_recommendation(candidates))
    }

    /// Analyze tools and generate recommendations
    pub fn recommend_tools(&self) -> Result<Recommendation> {
        let tool_names = self.storage.get_component_names(EventType::ToolUsage)?;
        let mut candidates = Vec::new();

        for name in tool_names {
            if let Ok(stats) = self
                .storage
                .get_component_stats(&name, EventType::ToolUsage)
            {
                let recommended = self.should_recommend(&stats);
                let reason = self.explain_recommendation(&stats, recommended);

                candidates.push(BuiltinCandidate {
                    name,
                    component_type: "tool".to_string(),
                    stats,
                    recommended,
                    reason,
                });
            }
        }

        Ok(self.build_recommendation(candidates))
    }

    /// Analyze MCPs and generate recommendations
    pub fn recommend_mcps(&self) -> Result<Recommendation> {
        let mcp_names = self.storage.get_component_names(EventType::McpUsage)?;
        let mut candidates = Vec::new();

        for name in mcp_names {
            if let Ok(stats) = self.storage.get_component_stats(&name, EventType::McpUsage) {
                let recommended = self.should_recommend(&stats);
                let reason = self.explain_recommendation(&stats, recommended);

                candidates.push(BuiltinCandidate {
                    name,
                    component_type: "mcp".to_string(),
                    stats,
                    recommended,
                    reason,
                });
            }
        }

        Ok(self.build_recommendation(candidates))
    }

    /// Generate recommendations for all component types
    pub fn generate_recommendation(&self) -> Result<Recommendation> {
        let skills_rec = self.recommend_skills()?;
        let tools_rec = self.recommend_tools()?;
        let mcps_rec = self.recommend_mcps()?;

        // Merge all candidates
        let mut all_candidates = Vec::new();
        all_candidates.extend(skills_rec.recommended);
        all_candidates.extend(skills_rec.not_recommended);
        all_candidates.extend(tools_rec.recommended);
        all_candidates.extend(tools_rec.not_recommended);
        all_candidates.extend(mcps_rec.recommended);
        all_candidates.extend(mcps_rec.not_recommended);

        Ok(self.build_recommendation(all_candidates))
    }

    /// Build recommendation result from candidates
    fn build_recommendation(&self, mut candidates: Vec<BuiltinCandidate>) -> Recommendation {
        // Sort by total calls (descending)
        candidates.sort_by(|a, b| b.stats.total_calls.cmp(&a.stats.total_calls));

        let recommended: Vec<BuiltinCandidate> = candidates
            .iter()
            .filter(|c| c.recommended)
            .cloned()
            .collect();

        let not_recommended: Vec<BuiltinCandidate> = candidates
            .iter()
            .filter(|c| !c.recommended)
            .cloned()
            .collect();

        Recommendation {
            recommended,
            not_recommended,
            generated_at: Utc::now(),
            thresholds: self.thresholds.clone(),
        }
    }

    /// Explain why a component is recommended or not
    fn explain_recommendation(&self, stats: &ComponentStats, recommended: bool) -> String {
        let success_rate = stats.success_rate();

        if recommended {
            format!(
                "Recommended: {} calls (threshold: {}), {:.1}% success (threshold: {:.1}%), {:.1}ms avg latency (threshold: {:.1}ms)",
                stats.total_calls,
                self.thresholds.min_calls_per_30d,
                success_rate * 100.0,
                self.thresholds.min_success_rate * 100.0,
                stats.avg_duration_ms,
                self.thresholds.max_latency_ms
            )
        } else {
            let mut reasons = Vec::new();

            if stats.total_calls < self.thresholds.min_calls_per_30d {
                reasons.push(format!(
                    "insufficient calls ({} < {})",
                    stats.total_calls, self.thresholds.min_calls_per_30d
                ));
            }

            if success_rate < self.thresholds.min_success_rate {
                reasons.push(format!(
                    "low success rate ({:.1}% < {:.1}%)",
                    success_rate * 100.0,
                    self.thresholds.min_success_rate * 100.0
                ));
            }

            if stats.avg_duration_ms > self.thresholds.max_latency_ms {
                reasons.push(format!(
                    "high latency ({:.1}ms > {:.1}ms)",
                    stats.avg_duration_ms, self.thresholds.max_latency_ms
                ));
            }

            format!("Not recommended: {}", reasons.join(", "))
        }
    }
}

impl Recommendation {
    /// Get total number of candidates
    pub fn total_candidates(&self) -> usize {
        self.recommended.len() + self.not_recommended.len()
    }

    /// Get recommendation summary
    pub fn summary(&self) -> String {
        format!(
            "{} of {} components recommended for builtin integration (as of {})",
            self.recommended.len(),
            self.total_candidates(),
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::TelemetryConfig;
    use tempfile::TempDir;

    fn test_recommender() -> BuiltinRecommender {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_rec.db");
        let config = TelemetryConfig {
            enabled: true,
            db_path,
        };
        let storage = TelemetryStorage::open(&config).unwrap();
        BuiltinRecommender::new(storage)
    }

    fn populate_test_data(storage: &TelemetryStorage) {
        // Popular skill (should be recommended)
        for i in 0..150 {
            let event = crate::telemetry::storage::TelemetryEvent {
                event_type: EventType::SkillUsage,
                component_name: "popular_skill".to_string(),
                metadata: None,
                timestamp: Utc::now(),
                success: i < 145,      // 96.7% success rate
                duration_ms: Some(80), // Below threshold
            };
            storage.record_event(&event).unwrap();
        }

        // Unpopular skill (not enough calls)
        for _ in 0..50 {
            let event = crate::telemetry::storage::TelemetryEvent {
                event_type: EventType::SkillUsage,
                component_name: "unpopular_skill".to_string(),
                metadata: None,
                timestamp: Utc::now(),
                success: true,
                duration_ms: Some(50),
            };
            storage.record_event(&event).unwrap();
        }

        // High latency skill
        for _ in 0..150 {
            let event = crate::telemetry::storage::TelemetryEvent {
                event_type: EventType::SkillUsage,
                component_name: "slow_skill".to_string(),
                metadata: None,
                timestamp: Utc::now(),
                success: true,
                duration_ms: Some(150), // Above threshold
            };
            storage.record_event(&event).unwrap();
        }
    }

    #[test]
    fn test_thresholds_default() {
        let thresholds = RecommendationThresholds::default();
        assert_eq!(thresholds.min_calls_per_30d, 100);
        assert_eq!(thresholds.min_success_rate, 0.95);
        assert_eq!(thresholds.max_latency_ms, 100.0);
    }

    #[test]
    fn test_should_recommend_all_pass() {
        let recommender = test_recommender();
        let stats = ComponentStats {
            total_calls: 150,
            successful_calls: 145,
            avg_duration_ms: 80.0,
        };
        assert!(recommender.should_recommend(&stats));
    }

    #[test]
    fn test_should_recommend_insufficient_calls() {
        let recommender = test_recommender();
        let stats = ComponentStats {
            total_calls: 50,
            successful_calls: 50,
            avg_duration_ms: 50.0,
        };
        assert!(!recommender.should_recommend(&stats));
    }

    #[test]
    fn test_should_recommend_low_success() {
        let recommender = test_recommender();
        let stats = ComponentStats {
            total_calls: 150,
            successful_calls: 100, // 66.7% success rate
            avg_duration_ms: 50.0,
        };
        assert!(!recommender.should_recommend(&stats));
    }

    #[test]
    fn test_should_recommend_high_latency() {
        let recommender = test_recommender();
        let stats = ComponentStats {
            total_calls: 150,
            successful_calls: 150,
            avg_duration_ms: 150.0,
        };
        assert!(!recommender.should_recommend(&stats));
    }

    #[test]
    fn test_recommend_skills() {
        let recommender = test_recommender();
        populate_test_data(&recommender.storage);

        let rec = recommender.recommend_skills().unwrap();
        assert_eq!(rec.recommended.len(), 1);
        assert_eq!(rec.recommended[0].name, "popular_skill");
        assert!(rec.recommended[0].recommended);
        assert_eq!(rec.not_recommended.len(), 2);
    }

    #[test]
    fn test_recommend_tools_empty() {
        let recommender = test_recommender();
        let rec = recommender.recommend_tools().unwrap();
        assert_eq!(rec.recommended.len(), 0);
        assert_eq!(rec.not_recommended.len(), 0);
    }

    #[test]
    fn test_generate_recommendation() {
        let recommender = test_recommender();
        populate_test_data(&recommender.storage);

        let rec = recommender.generate_recommendation().unwrap();
        assert_eq!(rec.recommended.len(), 1);
        assert!(rec.total_candidates() > 0);
    }

    #[test]
    fn test_recommendation_summary() {
        let recommender = test_recommender();
        populate_test_data(&recommender.storage);

        let rec = recommender.recommend_skills().unwrap();
        let summary = rec.summary();
        assert!(summary.contains("1 of 3"));
        assert!(summary.contains("recommended"));
    }

    #[test]
    fn test_explain_recommendation_pass() {
        let recommender = test_recommender();
        let stats = ComponentStats {
            total_calls: 150,
            successful_calls: 145,
            avg_duration_ms: 80.0,
        };
        let reason = recommender.explain_recommendation(&stats, true);
        assert!(reason.contains("Recommended"));
    }

    #[test]
    fn test_explain_recommendation_fail() {
        let recommender = test_recommender();
        let stats = ComponentStats {
            total_calls: 50,
            successful_calls: 50,
            avg_duration_ms: 50.0,
        };
        let reason = recommender.explain_recommendation(&stats, false);
        assert!(reason.contains("Not recommended"));
        assert!(reason.contains("insufficient calls"));
    }

    #[test]
    fn test_custom_thresholds() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_custom.db");
        let config = TelemetryConfig {
            enabled: true,
            db_path,
        };
        let storage = TelemetryStorage::open(&config).unwrap();

        let thresholds = RecommendationThresholds {
            min_calls_per_30d: 10,
            min_success_rate: 0.5,
            max_latency_ms: 200.0,
        };

        let recommender = BuiltinRecommender::with_thresholds(storage, thresholds);

        // With these loose thresholds, almost anything should pass
        let stats = ComponentStats {
            total_calls: 15,
            successful_calls: 10,
            avg_duration_ms: 150.0,
        };
        assert!(recommender.should_recommend(&stats));
    }
}
