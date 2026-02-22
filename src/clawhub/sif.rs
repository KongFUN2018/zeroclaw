use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Access levels for permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccessLevel {
    None,
    Readonly,
    Write,
    Full,
}

/// Permission declarations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permissions {
    pub network: AccessLevel,
    pub filesystem: AccessLevel,
    pub subprocess: bool,
    pub env_access: bool,
    pub max_memory_mb: Option<u32>,
    pub max_execution_seconds: Option<u32>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            network: AccessLevel::None,
            filesystem: AccessLevel::None,
            subprocess: false,
            env_access: false,
            max_memory_mb: None,
            max_execution_seconds: None,
        }
    }
}

/// Logic type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Logic {
    Prompt(PromptLogic),
    Code(CodeLogic),
    Hybrid(HybridLogic),
    Chain(ChainLogic),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptLogic {
    pub system: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeLogic {
    pub runtime: String,
    pub entrypoint: Option<String>,
    #[serde(rename = "function")]
    pub function_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Default for CodeLogic {
    fn default() -> Self {
        Self {
            runtime: String::new(),
            entrypoint: None,
            function_name: None,
            source: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HybridLogic {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_process: Option<CodeLogic>,
    pub main: Box<Logic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_process: Option<CodeLogic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChainLogic {
    pub steps: Vec<ChainStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChainStep {
    pub skill: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_inputs: Option<HashMap<String, serde_json::Value>>,
}

/// Parameter type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    String,
    Int,
    Float,
    Bool,
    Array,
    Object,
    Map,
    File,
    Any,
}

/// Input parameter definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InputParam {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: ParamType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_schema: Option<Box<InputParam>>,
}

/// Output parameter definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputParam {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: ParamType,
}

/// Interface definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interface {
    pub inputs: Vec<InputParam>,
    pub outputs: Vec<OutputParam>,
}

/// Dependency declaration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// Test case
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestCase {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<serde_json::Value>,
}

/// Compatibility declaration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Compatibility {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zeroclaw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openclaw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nanobot: Option<String>,
}

/// Signature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Signature {
    pub algorithm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_signature: Option<String>,
}

/// Language-agnostic skill representation for cross-project compatibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillSIF {
    pub metadata: SifMetadata,
    pub tools: Vec<ToolSIF>,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logic: Option<Logic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<Interface>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<Dependency>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<TestCase>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<Compatibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
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
