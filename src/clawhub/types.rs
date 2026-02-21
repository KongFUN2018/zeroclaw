use serde::{Deserialize, Serialize};

/// Entry in the ClawHub skill index
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillIndexEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub downloads: u64,
    pub source_project: String,
    pub repository: String,
    pub tags: Vec<String>,
}

/// Root index structure for ClawHub
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClawHubIndex {
    pub skills: Vec<SkillIndexEntry>,
    pub last_updated: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_index_entry_serialize() {
        let entry = SkillIndexEntry {
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            version: "1.0.0".to_string(),
            downloads: 100,
            source_project: "zeroclaw".to_string(),
            repository: "https://github.com/test/repo".to_string(),
            tags: vec!["test".to_string(), "demo".to_string()],
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: SkillIndexEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry, parsed);
    }

    #[test]
    fn test_clawhub_index_serialize() {
        let index = ClawHubIndex {
            skills: vec![SkillIndexEntry {
                name: "test-skill".to_string(),
                description: "A test skill".to_string(),
                version: "1.0.0".to_string(),
                downloads: 100,
                source_project: "zeroclaw".to_string(),
                repository: "https://github.com/test/repo".to_string(),
                tags: vec!["test".to_string()],
            }],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&index).unwrap();
        let parsed: ClawHubIndex = serde_json::from_str(&json).unwrap();

        assert_eq!(index, parsed);
    }
}
