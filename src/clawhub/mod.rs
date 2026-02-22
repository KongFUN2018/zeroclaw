pub mod cache;
pub mod detectors;
pub mod error;
pub mod hotload;
pub mod registry;
pub mod scraper;
pub mod sif;
pub mod signature;
pub mod types;
pub mod security;

#[cfg(test)]
mod detector_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod sif_tests;
#[cfg(test)]
mod security_tests;

pub use cache::ClawHubCache;
pub use detectors::{Detector, DetectorRegistry, ZeroClawDetector};
pub use error::{ClawHubError, Result};
pub use hotload::SkillHotLoader;
pub use registry::ClawHubClient;
pub use scraper::ClawHubScraper;
pub use sif::{
    SifMetadata, SkillSIF, ToolSIF,
    // Extended SIF types
    AccessLevel, Permissions, Logic, PromptLogic, CodeLogic, HybridLogic, ChainLogic,
    ChainStep, ParamType, InputParam, OutputParam, Interface,
    Dependency, TestCase, Compatibility, Signature,
};
pub use signature::{SkillKeyPair, SignatureVerification, sign_skill, verify_skill_signature, verify_sif_signature};
pub use types::{ClawHubIndex, SkillIndexEntry};
pub use security::{SecurityScanner, SecurityReport, Finding, Severity, SecurityRule,
                  PromptSafetyRule, PermissionConsistencyRule, CodeExecutionRiskRule,
                  InterfaceBoundaryRule, DependencyChainRule};
