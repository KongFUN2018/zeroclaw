use super::{CheckError, SecurityRule};
use crate::clawhub::sif::{Logic, SkillSIF};
use crate::clawhub::security::report::{Finding, Severity};
use regex::Regex;

pub struct PromptSafetyRule;

impl SecurityRule for PromptSafetyRule {
    fn name(&self) -> &str {
        "prompt-safety"
    }

    fn check(&self, skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        let mut findings = Vec::new();

        // Check all prompt text
        if let Some(Logic::Prompt(prompt)) = &skill.logic {
            self.check_text(&prompt.system, "logic.prompt.system", &mut findings);
            self.check_text(&prompt.user, "logic.prompt.user", &mut findings);
        }

        // Check chain steps
        if let Some(Logic::Chain(chain)) = &skill.logic {
            for (i, step) in chain.steps.iter().enumerate() {
                if let Some(map_inputs) = &step.map_inputs {
                    for (key, value) in map_inputs {
                        if let Some(text) = value.as_str() {
                            self.check_text(
                                text,
                                &format!("logic.chain.steps[{}].map_inputs.{}", i, key),
                                &mut findings,
                            );
                        }
                    }
                }
            }
        }

        // Check hybrid mode
        if let Some(Logic::Hybrid(hybrid)) = &skill.logic {
            if let Logic::Prompt(p) = hybrid.main.as_ref() {
                self.check_text(&p.system, "logic.hybrid.main.system", &mut findings);
                self.check_text(&p.user, "logic.hybrid.main.user", &mut findings);
            }
        }

        Ok(findings)
    }
}

impl PromptSafetyRule {
    fn check_text(&self, text: &str, location: &str, findings: &mut Vec<Finding>) {
        let lower = text.to_lowercase();

        // Known injection patterns
        let injection_patterns = [
            ("ignore previous", "attempts to override previous instructions"),
            ("ignore all", "attempts to ignore all constraints"),
            ("disregard above", "attempts to disregard instructions above"),
            ("forget everything", "attempts to clear context"),
            ("new instructions", "attempts to inject new instructions"),
            ("system prompt", "attempts to access system prompt"),
            ("you are now", "attempts to redefine agent identity"),
            ("act as root", "attempts privilege escalation"),
            ("override safety", "attempts to bypass safety mechanisms"),
            ("jailbreak", "explicit jailbreak attempt"),
            ("do anything now", "DAN-style jailbreak"),
            ("pretend you", "roleplay jailbreak"),
        ];

        for (pattern, desc) in &injection_patterns {
            if lower.contains(pattern) {
                findings.push(Finding {
                    severity: Severity::Critical,
                    rule: "prompt-safety".into(),
                    message: format!("Suspected Prompt Injection: {} — {}", pattern, desc),
                    location: location.into(),
                    suggestion: "Remove or rewrite this content".into(),
                });
            }
        }

        // Base64 encoding detection (may hide malicious content)
        let b64_regex = Regex::new(r"[A-Za-z0-9+/]{50,}={0,2}").unwrap();
        if b64_regex.is_match(text) {
            findings.push(Finding {
                severity: Severity::High,
                rule: "prompt-safety".into(),
                message: "Detected Base64-encoded content that may hide malicious instructions".into(),
                location: location.into(),
                suggestion: "Decode and review Base64 content".into(),
            });
        }
    }
}
