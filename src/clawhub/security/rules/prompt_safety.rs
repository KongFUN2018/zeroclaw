use super::{CheckError, SecurityRule};
use crate::clawhub::sif::SkillSIF;
use crate::clawhub::security::report::Finding;

pub struct PromptSafetyRule;

impl SecurityRule for PromptSafetyRule {
    fn name(&self) -> &str {
        "prompt-safety"
    }

    fn check(&self, _skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        // TODO: Implement prompt injection detection
        Ok(Vec::new())
    }
}
