use serde::{Deserialize, Serialize};

/// Severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Security finding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub severity: Severity,
    pub rule: String,
    pub message: String,
    pub location: String,
    pub suggestion: String,
}

/// Security scan report
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityReport {
    pub findings: Vec<Finding>,
    pub scan_duration_ms: u64,
}

impl SecurityReport {
    /// Check if skill passes security scan
    pub fn is_safe(&self) -> bool {
        self.findings.is_empty()
            || self.findings.iter().all(|f| f.severity <= Severity::Medium)
    }

    /// Get findings by severity
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect()
    }
}
