pub mod prompt_safety;
pub mod permission;
pub mod code_safety;
pub mod interface;
pub mod dependency;

pub use prompt_safety::PromptSafetyRule;
pub use permission::PermissionConsistencyRule;
pub use code_safety::CodeExecutionRiskRule;
pub use interface::InterfaceBoundaryRule;
pub use dependency::DependencyChainRule;

use crate::clawhub::sif::SkillSIF;
use super::report::Finding;

/// Security rule trait
pub trait SecurityRule: Send + Sync {
    /// Rule name
    fn name(&self) -> &str;

    /// Check skill for security issues
    fn check(&self, skill: &SkillSIF) -> Result<Vec<Finding>, CheckError>;
}

/// Check error
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error("Invalid skill structure: {0}")]
    InvalidStructure(String),
}
