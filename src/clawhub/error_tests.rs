use super::error::{ClawHubError, Result};

#[test]
fn test_skill_not_found_error() {
    let err = ClawHubError::SkillNotFound {
        name: "test_skill".to_string(),
        location: "clawhub".to_string(),
    };

    assert!(matches!(err, ClawHubError::SkillNotFound { .. }));
    assert!(err.to_string().contains("test_skill"));
}

#[test]
fn test_invalid_format_error() {
    let err = ClawHubError::InvalidFormat {
        file: "skill.toml".to_string(),
        reason: "missing name field".to_string(),
    };

    assert!(matches!(err, ClawHubError::InvalidFormat { .. }));
}

#[test]
fn test_conversion_error() {
    let err = ClawHubError::ConversionError {
        from: "openclaw".to_string(),
        to: "zeroclaw".to_string(),
        reason: "incompatible tool types".to_string(),
    };

    assert!(matches!(err, ClawHubError::ConversionError { .. }));
}
