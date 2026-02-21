pub mod cache;
pub mod detectors;
pub mod error;
pub mod registry;
pub mod sif;
pub mod types;

#[cfg(test)]
mod detector_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod sif_tests;

pub use cache::ClawHubCache;
pub use detectors::{Detector, DetectorRegistry, ZeroClawDetector};
pub use error::{ClawHubError, Result};
pub use registry::ClawHubClient;
pub use sif::{SifMetadata, SkillSIF, ToolSIF};
pub use types::{ClawHubIndex, SkillIndexEntry};
