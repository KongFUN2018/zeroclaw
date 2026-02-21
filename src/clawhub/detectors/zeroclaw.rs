use super::Detector;
use crate::clawhub::{ClawHubError, Result, SifMetadata, SkillSIF};
use async_trait::async_trait;
use std::path::Path;

pub struct ZeroClawDetector;

impl ZeroClawDetector {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }
}

#[async_trait]
impl Detector for ZeroClawDetector {
    async fn can_detect(&self, path: &Path) -> bool {
        let skill_toml = path.join("SKILL.toml");
        skill_toml.exists()
    }

    async fn parse_to_sif(&self, path: &Path) -> Result<SkillSIF> {
        let skill_toml = path.join("SKILL.toml");

        // Read and parse SKILL.toml
        let content = tokio::fs::read_to_string(&skill_toml).await?;
        let skill_config: toml::Value =
            toml::from_str(&content).map_err(|e| ClawHubError::InvalidFormat {
                file: skill_toml.display().to_string(),
                reason: format!("Invalid TOML: {}", e),
            })?;

        // Extract metadata
        let metadata = SifMetadata {
            name: skill_config
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ClawHubError::InvalidFormat {
                    file: skill_toml.display().to_string(),
                    reason: "missing 'name' field".to_string(),
                })?
                .to_string(),
            version: skill_config
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            description: skill_config
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            author: skill_config
                .get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            source_project: "zeroclaw".to_string(),
            tags: skill_config
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        };

        // Read content.md if exists
        let content_path = path.join("content.md");
        let content = if content_path.exists() {
            tokio::fs::read_to_string(&content_path)
                .await
                .unwrap_or_default()
        } else {
            String::new()
        };

        // Parse tools (if any)
        let tools = Vec::new(); // TODO: Parse tools from SKILL.toml

        Ok(SkillSIF {
            metadata,
            tools,
            content,
        })
    }

    fn name(&self) -> &str {
        "zeroclaw"
    }
}
