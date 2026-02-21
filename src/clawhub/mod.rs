pub mod detectors;
pub mod error;
pub mod sif;

#[cfg(test)]
mod detector_tests;
#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod sif_tests;

pub use detectors::{Detector, DetectorRegistry, ZeroClawDetector};
pub use error::{ClawHubError, Result};
pub use sif::{SifMetadata, SkillSIF, ToolSIF};
