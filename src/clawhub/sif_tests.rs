use super::sif::{SifMetadata, SkillSIF, ToolSIF};

#[test]
fn test_sif_serialization() {
    let sif = SkillSIF {
        metadata: SifMetadata {
            name: "test_skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test skill".to_string(),
            author: "Test Author".to_string(),
            source_project: "zeroclaw".to_string(),
            tags: vec!["test".to_string(), "demo".to_string()],
        },
        tools: vec![ToolSIF {
            name: "test_tool".to_string(),
            description: "Test tool".to_string(),
            implementation: serde_json::json!({
                "type": "bash",
                "command": "echo 'hello'"
            }),
        }],
        content: "Test content".to_string(),
    };

    let json = serde_json::to_string(&sif).unwrap();
    assert!(json.contains("test_skill"));

    let deserialized: SkillSIF = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.metadata.name, "test_skill");
}

#[test]
fn test_sif_cross_project_compatibility() {
    // Test that SIF can represent skills from different projects
    let openclaw_sif = SkillSIF {
        metadata: SifMetadata {
            name: "openclaw_skill".to_string(),
            version: "2.0.0".to_string(),
            description: "From OpenClaw".to_string(),
            author: "OpenClaw Team".to_string(),
            source_project: "openclaw".to_string(),
            tags: vec!["typescript".to_string()],
        },
        tools: vec![],
        content: "OpenClaw content".to_string(),
    };

    assert_eq!(openclaw_sif.metadata.source_project, "openclaw");
}
