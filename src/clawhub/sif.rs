use serde::{Deserialize, Serialize};

/// Language-agnostic skill representation for cross-project compatibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillSIF {
    pub metadata: SifMetadata,
    pub tools: Vec<ToolSIF>,
    pub content: String,
}

/// Standardized metadata across all Claw projects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SifMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub source_project: String, // "openclaw", "nanobot", "picoclaw", etc.
    pub tags: Vec<String>,
}

/// Platform-specific tool implementation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSIF {
    pub name: String,
    pub description: String,
    /// Flexible implementation to support different platforms
    pub implementation: serde_json::Value,
}

impl SkillSIF {
    /// Validate SIF structure
    pub fn validate(&self) -> super::Result<()> {
        if self.metadata.name.is_empty() {
            return Err(super::ClawHubError::InvalidFormat {
                file: "SIF".to_string(),
                reason: "name cannot be empty".to_string(),
            });
        }

        if self.metadata.version.is_empty() {
            return Err(super::ClawHubError::InvalidFormat {
                file: "SIF".to_string(),
                reason: "version cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}
