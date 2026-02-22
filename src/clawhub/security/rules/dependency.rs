use super::{CheckError, SecurityRule};
use crate::clawhub::sif::{Logic, SkillSIF};
use crate::clawhub::security::report::{Finding, Severity};
use std::collections::HashSet;

pub struct DependencyChainRule;

impl SecurityRule for DependencyChainRule {
    fn name(&self) -> &str {
        "dependency-chain"
    }

    fn check(&self, skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        let mut findings = Vec::new();

        // Check dependencies
        if let Some(deps) = &skill.dependencies {
            for (idx, dep) in deps.iter().enumerate() {
                let location = format!("dependencies[{}]", idx);

                // Check for missing hash
                if dep.hash.is_none() {
                    findings.push(Finding {
                        severity: Severity::Medium,
                        rule: "dependency-chain".into(),
                        message: format!(
                            "Dependency '{}' missing integrity hash",
                            dep.name
                        ),
                        location: location.clone(),
                        suggestion: "Add hash to verify dependency integrity".into(),
                    });
                }

                // Check for unbounded versions (no upper bound)
                if dep.version.starts_with(">=") || dep.version.starts_with("^") {
                    // Check if there's an explicit upper bound
                    if !dep.version.contains('<') && !dep.version.contains(',') {
                        findings.push(Finding {
                            severity: Severity::Medium,
                            rule: "dependency-chain".into(),
                            message: format!(
                                "Dependency '{}' version '{}' has no upper bound, may break with future versions",
                                dep.name, dep.version
                            ),
                            location: location.clone(),
                            suggestion: "Use specific version range with upper bound".into(),
                        });
                    }
                }
            }
        }

        // Check chain steps reference declared dependencies
        if let Some(Logic::Chain(chain)) = &skill.logic {
            let declared_deps: HashSet<&str> = skill
                .dependencies
                .as_ref()
                .map(|deps| deps.iter().map(|d| d.name.as_str()).collect())
                .unwrap_or_default();

            for (idx, step) in chain.steps.iter().enumerate() {
                let location = format!("logic.chain.steps[{}]", idx);

                if !declared_deps.contains(step.skill.as_str()) {
                    findings.push(Finding {
                        severity: Severity::High,
                        rule: "dependency-chain".into(),
                        message: format!(
                            "Chain step references undeclared skill '{}'",
                            step.skill
                        ),
                        location: location.clone(),
                        suggestion: "Add skill to dependencies or verify chain step".into(),
                    });
                }
            }
        }

        Ok(findings)
    }
}
