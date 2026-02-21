use super::{CheckError, SecurityRule};
use crate::clawhub::sif::SkillSIF;
use crate::clawhub::security::report::Finding;

pub struct DependencyChainRule;

impl SecurityRule for DependencyChainRule {
    fn name(&self) -> &str {
        "dependency-chain"
    }

    fn check(&self, _skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        // TODO: Implement dependency chain checks
        Ok(Vec::new())
    }
}
