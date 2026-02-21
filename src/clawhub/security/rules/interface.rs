use super::{CheckError, SecurityRule};
use crate::clawhub::sif::SkillSIF;
use crate::clawhub::security::report::Finding;

pub struct InterfaceBoundaryRule;

impl SecurityRule for InterfaceBoundaryRule {
    fn name(&self) -> &str {
        "interface-boundary"
    }

    fn check(&self, _skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        // TODO: Implement interface boundary checks
        Ok(Vec::new())
    }
}
