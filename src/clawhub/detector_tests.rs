use crate::clawhub::detectors::{Detector, DetectorRegistry, ZeroClawDetector};
use std::path::Path;

#[tokio::test]
async fn test_zeroclaw_detector_detects_skill() {
    let detector = ZeroClawDetector::new();
    let skill_path = Path::new("tests/fixtures/skills/valid_skill");

    let detected = detector.can_detect(skill_path).await;
    assert!(detected, "Should detect ZeroClaw skill");
}

#[tokio::test]
async fn test_zeroclaw_detector_parses_to_sif() {
    let detector = ZeroClawDetector::new();
    let skill_path = Path::new("tests/fixtures/skills/valid_skill");

    let sif = detector.parse_to_sif(skill_path).await.unwrap();
    assert_eq!(sif.metadata.source_project, "zeroclaw");
    assert!(!sif.metadata.name.is_empty());
}

#[test]
fn test_detector_registry() {
    let registry = DetectorRegistry::with_defaults();
    assert!(!registry.detectors.is_empty());
}
