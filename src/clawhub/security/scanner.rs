use crate::clawhub::sif::SkillSIF;
use super::{report::SecurityReport, rules::SecurityRule};
use std::sync::Arc;
use std::time::Instant;

/// Security scanner
pub struct SecurityScanner {
    rules: Vec<Arc<dyn SecurityRule>>,
}

impl SecurityScanner {
    /// Create new scanner with default rules
    pub fn new() -> Self {
        use super::rules::*;

        Self {
            rules: vec![
                Arc::new(PromptSafetyRule),
                Arc::new(PermissionConsistencyRule),
                Arc::new(CodeExecutionRiskRule),
                Arc::new(InterfaceBoundaryRule),
                Arc::new(DependencyChainRule),
            ],
        }
    }

    /// Get number of rules
    pub fn rules(&self) -> &[Arc<dyn SecurityRule>] {
        &self.rules
    }

    /// Check if scanner has specific rule
    pub fn has_rule(&self, name: &str) -> bool {
        self.rules.iter().any(|r| r.name() == name)
    }

    /// Scan skill for security issues
    pub fn scan(&self, skill: &SkillSIF) -> Result<SecurityReport, ScanError> {
        let start = Instant::now();
        let mut findings = Vec::new();

        for rule in &self.rules {
            match rule.check(skill) {
                Ok(rule_findings) => findings.extend(rule_findings),
                Err(e) => return Err(ScanError::RuleFailed {
                    rule: rule.name().into(),
                    reason: e.to_string(),
                }),
            }
        }

        findings.sort_by_key(|f| std::cmp::Reverse(f.severity));

        Ok(SecurityReport {
            findings,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Scan error
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("Rule '{rule}' failed: {reason}")]
    RuleFailed { rule: String, reason: String },
}
