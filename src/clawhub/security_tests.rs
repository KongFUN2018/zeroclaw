use super::security::*;
use super::sif::*;

#[test]
fn test_scanner_creation() {
    let scanner = SecurityScanner::new();
    assert_eq!(scanner.rules().len(), 5);
    assert!(scanner.has_rule("prompt-safety"));
    assert!(scanner.has_rule("permission-consistency"));
    assert!(scanner.has_rule("code-execution-risk"));
    assert!(scanner.has_rule("interface-boundary"));
    assert!(scanner.has_rule("dependency-chain"));
}

#[test]
fn test_scan_safe_skill() {
    let scanner = SecurityScanner::new();
    let safe_skill = create_safe_skill();

    let report = scanner.scan(&safe_skill).unwrap();
    assert_eq!(report.findings.len(), 0);
    assert!(report.is_safe());
}

fn create_safe_skill() -> SkillSIF {
    SkillSIF {
        metadata: SifMetadata {
            name: "safe_skill".to_string(),
            version: "1.0.0".to_string(),
            description: "A safe skill".to_string(),
            author: "test@example.com".to_string(),
            source_project: "zeroclaw".to_string(),
            tags: vec![],
        },
        tools: vec![],
        content: "Safe content".to_string(),
        permissions: None,
        logic: Some(Logic::Prompt(PromptLogic {
            system: "You are a helpful assistant.".to_string(),
            user: "Please help the user.".to_string(),
            model_hint: None,
            temperature: None,
        })),
        interface: None,
        dependencies: None,
        tests: None,
        compatibility: None,
        signature: None,
    }
}
