use super::sif::*;
use crate::clawhub::sif::{SifMetadata, SkillSIF, ToolSIF};

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
        permissions: None,
        logic: None,
        interface: None,
        dependencies: None,
        tests: None,
        compatibility: None,
        signature: None,
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
        permissions: None,
        logic: None,
        interface: None,
        dependencies: None,
        tests: None,
        compatibility: None,
        signature: None,
    };

    assert_eq!(openclaw_sif.metadata.source_project, "openclaw");
}

#[test]
fn test_extended_sif_with_permissions() {
    let sif_json = r#"{
        "metadata": {
            "name": "test-skill",
            "version": "1.0.0",
            "description": "Test skill with permissions",
            "author": "test@example.com",
            "source_project": "zeroclaw",
            "tags": ["test"]
        },
        "tools": [],
        "content": "test content",
        "permissions": {
            "network": "none",
            "filesystem": "readonly",
            "subprocess": false,
            "env_access": false,
            "max_memory_mb": 256,
            "max_execution_seconds": 30
        },
        "logic": {
            "type": "prompt",
            "system": "You are a helpful assistant",
            "user": "Please help with {{input}}"
        },
        "interface": {
            "inputs": [{
                "name": "input",
                "type": "string",
                "required": true,
                "max_length": 1000
            }],
            "outputs": [{
                "name": "result",
                "type": "string"
            }]
        },
        "dependencies": [],
        "tests": [],
        "compatibility": {},
        "signature": null
    }"#;

    let sif: SkillSIF = serde_json::from_str(sif_json).unwrap();

    assert_eq!(sif.metadata.name, "test-skill");
    assert_eq!(sif.permissions.as_ref().unwrap().network, AccessLevel::None);
    assert_eq!(sif.permissions.as_ref().unwrap().filesystem, AccessLevel::Readonly);
    assert_eq!(sif.logic.as_ref().unwrap(), &Logic::Prompt(PromptLogic {
        system: "You are a helpful assistant".into(),
        user: "Please help with {{input}}".into(),
        model_hint: None,
        temperature: None
    }));
}
