//! Claw skill format detectors
//!
//! This module provides detectors for different Claw project skill formats.
//! Each detector implements the `Detector` trait to identify and parse
//! skills from various Claw projects (OpenClaw, NanoBot, PicoClaw, NullClaw, etc.)
//! into the unified Skill Intermediate Format (SIF).
//!
//! # Current Implementation
//!
//! - **ZeroClawDetector**: Parses ZeroClaw's SKILL.toml format
//!
//! # Future Detectors (Out of Scope for Initial Implementation)
//!
//! The following detectors are planned but not yet implemented:
//!
//! - **OpenClawDetector**: TypeScript/YAML-based skill format
//! - **NanoBotDetector**: Python dict-based skill format
//! - **PicoClawDetector**: Go/YAML-based skill format
//! - **NullClawDetector**: Zig/JSON-based skill format
//!
//! These will be added as needed when actual cross-project skill sharing
//! becomes a requirement. The architecture supports easy addition via
//! the `Detector` trait.

use crate::clawhub::{Result, SkillSIF};
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait Detector: Send + Sync {
    /// Check if this detector can handle the given path
    async fn can_detect(&self, path: &Path) -> bool;

    /// Parse skill at path to SIF format
    async fn parse_to_sif(&self, path: &Path) -> Result<SkillSIF>;

    /// Detector name for logging
    fn name(&self) -> &str {
        "unknown"
    }
}

pub struct DetectorRegistry {
    pub detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    /// Create a registry with default detectors
    pub fn with_defaults() -> Self {
        Self::default()
    }

    pub fn register(mut self, detector: Box<dyn Detector>) -> Self {
        self.detectors.push(detector);
        self
    }

    pub async fn detect_and_parse(&self, path: &Path) -> Result<SkillSIF> {
        for detector in &self.detectors {
            if detector.can_detect(path).await {
                tracing::info!("Detected {} format at {}", detector.name(), path.display());
                return detector.parse_to_sif(path).await;
            }
        }

        Err(crate::clawhub::ClawHubError::InvalidFormat {
            file: path.display().to_string(),
            reason: "No detector recognized this format".to_string(),
        })
    }
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self::new().register(ZeroClawDetector::new())
    }
}

pub mod zeroclaw;

pub use zeroclaw::ZeroClawDetector;
