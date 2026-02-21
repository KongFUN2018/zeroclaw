use super::{CheckError, SecurityRule};
use crate::clawhub::sif::SkillSIF;
use crate::clawhub::security::report::Finding;

pub struct CodeExecutionRiskRule;

impl SecurityRule for CodeExecutionRiskRule {
    fn name(&self) -> &str {
        "code-execution-risk"
    }

    fn check(&self, _skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        // TODO: Implement code execution risk checks
        Ok(Vec::new())
    }
}
