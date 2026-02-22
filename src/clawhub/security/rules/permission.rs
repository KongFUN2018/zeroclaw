use super::{CheckError, SecurityRule};
use crate::clawhub::sif::{Logic, Permissions, SkillSIF};
use crate::clawhub::security::report::{Finding, Severity};

pub struct PermissionConsistencyRule;

impl SecurityRule for PermissionConsistencyRule {
    fn name(&self) -> &str {
        "permission-consistency"
    }

    fn check(&self, skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        let mut findings = Vec::new();

        let default_permissions = Permissions::default();
        let permissions = skill.permissions.as_ref().unwrap_or(&default_permissions);
        let prompt_text = extract_all_text(skill);
        let lower = prompt_text.to_lowercase();

        // Declares no network but prompt hints at network access
        if matches!(permissions.network, crate::clawhub::sif::AccessLevel::None) {
            let network_hints = [
                "fetch", "http", "https", "api call", "request url",
                "download", "upload", "send request", "curl", "wget",
            ];
            for hint in &network_hints {
                if lower.contains(hint) {
                    findings.push(Finding {
                        severity: Severity::High,
                        rule: "permission-consistency".into(),
                        message: format!(
                            "Permission declares network=none, but prompt contains network operation hint: '{}'",
                            hint
                        ),
                        location: "permissions.network vs logic".into(),
                        suggestion: "Update permission to network=readonly/full or remove network-related instructions".into(),
                    });
                }
            }
        }

        // Declares no filesystem but prompt hints at file operations
        if matches!(permissions.filesystem, crate::clawhub::sif::AccessLevel::None) {
            let fs_hints = [
                "read", "write", "open file", "save",
                "load", "file path", "directory",
            ];
            for hint in &fs_hints {
                if lower.contains(hint) {
                    findings.push(Finding {
                        severity: Severity::High,
                        rule: "permission-consistency".into(),
                        message: format!(
                            "Permission declares filesystem=none, but prompt contains file operation hint: '{}'",
                            hint
                        ),
                        location: "permissions.filesystem vs logic".into(),
                        suggestion: "Update permission declaration or remove file operation instructions".into(),
                    });
                }
            }
        }

        Ok(findings)
    }
}

fn extract_all_text(skill: &SkillSIF) -> String {
    let mut text = String::new();

    if let Some(Logic::Prompt(prompt)) = &skill.logic {
        text.push_str(&prompt.system);
        text.push_str(&prompt.user);
    }

    if let Some(Logic::Chain(chain)) = &skill.logic {
        for step in &chain.steps {
            if let Some(map_inputs) = &step.map_inputs {
                for value in map_inputs.values() {
                    if let Some(s) = value.as_str() {
                        text.push_str(s);
                    }
                }
            }
        }
    }

    text
}
