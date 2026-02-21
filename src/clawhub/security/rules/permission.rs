use super::{CheckError, SecurityRule};
use crate::clawhub::sif::SkillSIF;
use crate::clawhub::security::report::Finding;

pub struct PermissionConsistencyRule;

impl SecurityRule for PermissionConsistencyRule {
    fn name(&self) -> &str {
        "permission-consistency"
    }

    fn check(&self, _skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        // TODO: Implement permission consistency checks
        Ok(Vec::new())
    }
}
