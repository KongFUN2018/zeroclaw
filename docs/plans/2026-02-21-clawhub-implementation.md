# ClawHub Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a comprehensive Claw ecosystem integration enabling ZeroClaw to use 700+ skills from OpenClaw, NanoBot, PicoClaw, NullClaw, and other Claw projects with intelligent selection, autonomous installation, telemetry-driven self-improvement, and smart builtin recommendations.

**Architecture:**
1. **Skill Intermediate Format (SIF)** - Language-agnostic representation for cross-project compatibility
2. **Detector Registry** - Auto-detect and parse different Claw project formats
3. **Hot Loader** - File system watching for instant skill updates without restart
4. **Intelligent Selector** - Complexity analysis + LLM-based skill selection with progressive disclosure
5. **Telemetry System** - SQLite-based usage tracking and optimization engine
6. **Smart Builtin** - Recommend frequently-used components for core integration

**Tech Stack:**
- Rust (async/await, tokio)
- SQLite (rusqlite) for telemetry storage
- notify crate for file system watching
- reqwest for HTTP API calls
- serde/serde_json for serialization
- tracing for logging

---

## Implementation Phases

**Phase 1: Core Infrastructure** (SIF, detectors, error types)
**Phase 2: ClawHub Client & CLI Integration** (API, cache, commands)
**Phase 3: Converters & Hot Loader** (Format conversion, file watching)
**Phase 4: Intelligent Skill Selector** (Complexity analysis, progressive disclosure)
**Phase 5: Telemetry System** (Collection, storage, query, optimization)
**Phase 6: Smart Builtin Recommendation** (Detection, analysis, user notification)

---

## Phase 1: Core Infrastructure

### Task 1.1: Create ClawHub module structure and error types

**Files:**
- Create: `src/clawhub/mod.rs`
- Create: `src/clawhub/error.rs`
- Modify: `src/lib.rs` (add module export)
- Test: `src/clawhub/tests/error_tests.rs`

**Step 1: Write the failing test**

Create `src/clawhub/tests/error_tests.rs`:

```rust
use zeroclaw::clawhub::error::{ClawHubError, Result};

#[test]
fn test_skill_not_found_error() {
    let err = ClawHubError::SkillNotFound {
        name: "test_skill".to_string(),
        source: "clawhub".to_string(),
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
```

**Step 2: Run test to verify it fails**

```bash
cargo test --package zeroclaw clawhub::tests::error_tests
```

Expected: Compilation error "module `clawhub` not found"

**Step 3: Create error types**

Create `src/clawhub/error.rs`:

```rust
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ClawHubError {
    #[error("Skill '{name}' not found in {source}")]
    SkillNotFound { name: String, source: String },

    #[error("Invalid format in {file}: {reason}")]
    InvalidFormat { file: String, reason: String },

    #[error("Conversion error from {from} to {to}: {reason}")]
    ConversionError { from: String, to: String, reason: String },

    #[error("Missing dependency: {dependency}")]
    DependencyMissing { dependency: String },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Path traversal detected: {path}")]
    PathTraversal { path: String },
}

pub type Result<T> = std::result::Result<T, ClawHubError>;
```

**Step 4: Create module entry**

Create `src/clawhub/mod.rs`:

```rust
pub mod error;

pub use error::{ClawHubError, Result};
```

**Step 5: Export from lib**

Add to `src/lib.rs`:

```rust
pub mod clawhub;
```

**Step 6: Run tests to verify they pass**

```bash
cargo test --package zeroclaw clawhub::tests::error_tests
```

Expected: All tests PASS

**Step 7: Commit**

```bash
git add src/clawhub/mod.rs src/clawhub/error.rs src/lib.rs src/clawhub/tests/
git commit -m "feat(clawhub): add core error types and module structure"
```

---

### Task 1.2: Implement Skill Intermediate Format (SIF)

**Files:**
- Create: `src/clawhub/sif.rs`
- Create: `src/clawhub/tests/sif_tests.rs`

**Step 1: Write the failing test**

Create `src/clawhub/tests/sif_tests.rs`:

```rust
use zeroclaw::clawhub::sif::{SkillSIF, ToolSIF, SifMetadata};

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
        tools: vec![
            ToolSIF {
                name: "test_tool".to_string(),
                description: "Test tool".to_string(),
                implementation: serde_json::json!({
                    "type": "bash",
                    "command": "echo 'hello'"
                }),
            }
        ],
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
```

**Step 2: Run test to verify it fails**

```bash
cargo test --package zeroclaw clawhub::tests::sif_tests
```

Expected: "module `sif` not found"

**Step 3: Implement SIF structure**

Create `src/clawhub/sif.rs`:

```rust
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
    pub source_project: String,  // "openclaw", "nanobot", "picoclaw", etc.
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
```

**Step 4: Update module exports**

Update `src/clawhub/mod.rs`:

```rust
pub mod error;
pub mod sif;

pub use error::{ClawHubError, Result};
pub use sif::{SkillSIF, SifMetadata, ToolSIF};
```

**Step 5: Run tests to verify they pass**

```bash
cargo test --package zeroclaw clawhub::tests::sif_tests
```

Expected: All tests PASS

**Step 6: Commit**

```bash
git add src/clawhub/sif.rs src/clawhub/mod.rs src/clawhub/tests/sif_tests.rs
git commit -m "feat(clawhub): implement Skill Intermediate Format (SIF)"
```

---

### Task 1.3: Create detector trait and ZeroClaw detector

**Files:**
- Create: `src/clawhub/detectors/mod.rs`
- Create: `src/clawhub/detectors/zeroclaw.rs`
- Test: `src/clawhub/tests/detector_tests.rs`

**Step 1: Write the failing test**

Create `src/clawhub/tests/detector_tests.rs`:

```rust
use zeroclaw::clawhub::detectors::{Detector, DetectorRegistry, ZeroClawDetector};
use zeroclaw::clawhub::sif::SkillSIF;
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
    let registry = DetectorRegistry::new();
    assert!(!registry.detectors.is_empty());
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --package zeroclaw clawhub::tests::detector_tests
```

Expected: "module `detectors` not found"

**Step 3: Create detector trait**

Create `src/clawhub/detectors/mod.rs`:

```rust
use async_trait::async_trait;
use std::path::Path;
use crate::clawhub::{Result, SkillSIF};

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
    detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
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
        Self::new()
    }
}

pub mod zeroclaw;
```

**Step 4: Implement ZeroClaw detector**

Create `src/clawhub/detectors/zeroclaw.rs`:

```rust
use super::Detector;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::clawhub::{Result, ClawHubError, SkillSIF, SifMetadata, ToolSIF};

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
        let skill_config: toml::Value = toml::from_str(&content)
            .map_err(|e| ClawHubError::InvalidFormat {
                file: skill_toml.display().to_string(),
                reason: format!("Invalid TOML: {}", e),
            })?;

        // Extract metadata
        let metadata = SifMetadata {
            name: skill_config.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ClawHubError::InvalidFormat {
                    file: skill_toml.display().to_string(),
                    reason: "missing 'name' field".to_string(),
                })?
                .to_string(),
            version: skill_config.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            description: skill_config.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            author: skill_config.get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            source_project: "zeroclaw".to_string(),
            tags: skill_config.get("tags")
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
            tokio::fs::read_to_string(&content_path).await.unwrap_or_default()
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
```

**Step 5: Update module exports**

Update `src/clawhub/detectors/mod.rs`:

```rust
pub use zeroclaw::ZeroClawDetector;
```

**Step 6: Create test fixture**

Create `tests/fixtures/skills/valid_skill/SKILL.toml`:

```toml
name = "test_skill"
version = "1.0.0"
description = "A test skill"
author = "Test Author"
tags = ["test", "demo"]
```

Create `tests/fixtures/skills/valid_skill/content.md`:

```markdown
# Test Skill Content

This is a test skill content.
```

**Step 7: Run tests to verify they pass**

```bash
cargo test --package zeroclaw clawhub::tests::detector_tests
```

Expected: All tests PASS

**Step 8: Commit**

```bash
git add src/clawhub/detectors/ src/clawhub/tests/detector_tests.rs tests/fixtures/skills/
git commit -m "feat(clawhub): add detector trait and ZeroClaw detector"
```

---

## Phase 2: ClawHub Client & CLI Integration

### Task 2.1: Implement ClawHub API client

**Files:**
- Create: `src/clawhub/registry.rs`
- Create: `src/clawhub/cache.rs`
- Test: `src/clawhub/tests/registry_tests.rs`

**Step 1: Write the failing test**

Create `src/clawhub/tests/registry_tests.rs`:

```rust
use zeroclaw::clawhub::registry::ClawHubClient;

#[tokio::test]
async fn test_client_search() {
    let client = ClawHubClient::new();
    let results = client.search("git").await.unwrap();

    // Should return some results (or empty if offline)
    assert!(results.len() >= 0);
}

#[tokio::test]
async fn test_client_get_trending() {
    let client = ClawHubClient::new();
    let trending = client.get_trending(10).await.unwrap();

    assert!(trending.len() <= 10);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --package zeroclaw clawhub::tests::registry_tests
```

Expected: "module `registry` not found"

**Step 3: Implement types**

Create `src/clawhub/types.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Skill metadata from ClawHub index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillIndexEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub downloads: u64,
    pub source_project: String,
    pub repository: String,
    pub tags: Vec<String>,
}

/// ClawHub index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawHubIndex {
    pub skills: Vec<SkillIndexEntry>,
    pub last_updated: String,
}
```

**Step 4: Implement cache**

Create `src/clawhub/cache.rs`:

```rust
use crate::clawhub::{Result, ClawHubError};
use std::path::PathBuf;
use tokio::fs;

pub struct ClawHubCache {
    cache_dir: PathBuf,
}

impl ClawHubCache {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| ClawHubError::IoError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
            ))?
            .join(".cache")
            .join("zeroclaw")
            .join("clawhub");

        fs::create_dir_all(&cache_dir).await?;

        Ok(Self { cache_dir })
    }

    pub async fn get_index(&self) -> Result<Option<crate::clawhub::types::ClawHubIndex>> {
        let index_path = self.cache_dir.join("index.json");

        if !index_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&index_path).await?;
        let index: crate::clawhub::types::ClawHubIndex = serde_json::from_str(&content)?;
        Ok(Some(index))
    }

    pub async fn save_index(&self, index: &crate::clawhub::types::ClawHubIndex) -> Result<()> {
        let index_path = self.cache_dir.join("index.json");
        let content = serde_json::to_string_pretty(index)?;
        fs::write(&index_path, content).await?;
        Ok(())
    }

    pub async fn is_index_fresh(&self, ttl_hours: u64) -> bool {
        let index_path = self.cache_dir.join("index.json");

        if !index_path.exists() {
            return false;
        }

        let metadata = fs::metadata(&index_path).await.ok()?;
        let modified = metadata.modified().ok()?;
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .ok()?;

        age.as_secs() < (ttl_hours * 3600)
    }
}
```

**Step 5: Implement registry client**

Create `src/clawhub/registry.rs`:

```rust
use crate::clawhub::{Result, ClawHubError, types::{ClawHubIndex, SkillIndexEntry}};
use super::cache::ClawHubCache;

pub struct ClawHubClient {
    cache: ClawHubCache,
    api_base: String,
}

impl ClawHubClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache: ClawHubCache::new()?,
            api_base: "https://api.clawhub.ai".to_string(),
        })
    }

    async fn fetch_index(&self) -> Result<ClawHubIndex> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/skills/index", self.api_base);

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClawHubError::NetworkError(
                reqwest::Error::from(
                    reqwest::StatusCode::BAD_REQUEST
                )
            ));
        }

        let index = response.json().await?;
        Ok(index)
    }

    pub async fn get_index(&self, ttl_hours: u64) -> Result<ClawHubIndex> {
        // Try cache first
        if self.cache.is_index_fresh(ttl_hours).await {
            if let Some(cached) = self.cache.get_index().await? {
                tracing::debug!("Using cached ClawHub index");
                return Ok(cached);
            }
        }

        // Fetch fresh index
        tracing::info!("Fetching fresh ClawHub index");
        let index = self.fetch_index().await?;
        self.cache.save_index(&index).await?;
        Ok(index)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SkillIndexEntry>> {
        let index = self.get_index(24).await?;

        let query_lower = query.to_lowercase();
        let results: Vec<SkillIndexEntry> = index.skills
            .into_iter()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&query_lower) ||
                skill.description.to_lowercase().contains(&query_lower) ||
                skill.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(results)
    }

    pub async fn get_trending(&self, limit: usize) -> Result<Vec<SkillIndexEntry>> {
        let index = self.get_index(24).await?;

        let mut skills = index.skills;
        skills.sort_by(|a, b| b.downloads.cmp(&a.downloads));

        Ok(skills.into_iter().take(limit).collect())
    }
}

impl Default for ClawHubClient {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
```

**Step 6: Update module exports**

Update `src/clawhub/mod.rs`:

```rust
pub mod cache;
pub mod detectors;
pub mod error;
pub mod registry;
pub mod sif;
pub mod types;

pub use cache::ClawHubCache;
pub use detectors::{Detector, DetectorRegistry, ZeroClawDetector};
pub use error::{ClawHubError, Result};
pub use registry::ClawHubClient;
pub use sif::{SkillSIF, SifMetadata, ToolSIF};
pub use types::{ClawHubIndex, SkillIndexEntry};
```

**Step 7: Run tests to verify they pass**

```bash
cargo test --package zeroclaw clawhub::tests::registry_tests
```

Expected: All tests PASS (may fail if network unavailable, that's OK)

**Step 8: Commit**

```bash
git add src/clawhub/registry.rs src/clawhub/cache.rs src/clawhub/types.rs src/clawhub/mod.rs src/clawhub/tests/registry_tests.rs
git commit -m "feat(clawhub): implement API client and cache"
```

---

### Task 2.2: Add CLI commands for skill management

**Files:**
- Create: `src/commands/skills.rs`
- Modify: `src/main.rs` (add skills command)
- Test: Manual testing

**Step 1: Create skills command module**

Create `src/commands/skills.rs`:

```rust
use clap::{Parser, Subcommand};
use crate::clawhub::ClawHubClient;

#[derive(Parser)]
pub struct SkillsCommand {
    #[clap(subcommand)]
    pub subcommand: SkillsSubcommand,
}

#[derive(Subcommand)]
pub enum SkillsSubcommand {
    /// Search for skills on ClawHub
    Search {
        /// Search query
        query: String,
    },

    /// Show trending skills
    Trending {
        /// Number of results to show
        #[clap(short, long, default_value = "10")]
        count: usize,
    },

    /// List installed skills
    List,

    /// Update skill index
    Update,

    /// Show skill information
    Info {
        /// Skill name
        name: String,
    },
}

impl SkillsCommand {
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.subcommand {
            SkillsSubcommand::Search { query } => {
                self.search(query).await
            }
            SkillsSubcommand::Trending { count } => {
                self.trending(*count).await
            }
            SkillsSubcommand::List => {
                self.list().await
            }
            SkillsSubcommand::Update => {
                self.update().await
            }
            SkillsSubcommand::Info { name } => {
                self.info(name).await
            }
        }
    }

    async fn search(&self, query: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = ClawHubClient::new()?;
        let results = client.search(query).await?;

        println!("Found {} skills matching '{}':", results.len(), query);
        println!();

        for skill in results {
            println!("  {} v{}", skill.name, skill.version);
            println!("    {}", skill.description);
            println!("    Downloads: {} | Tags: {}", skill.downloads, skill.tags.join(", "));
            println!();
        }

        Ok(())
    }

    async fn trending(&self, count: usize) -> Result<(), Box<dyn std::error::Error>> {
        let client = ClawHubClient::new()?;
        let trending = client.get_trending(*count).await?;

        println!("Top {} trending skills on ClawHub:", count);
        println!();

        for (i, skill) in trending.iter().enumerate() {
            println!("  {}. {} v{}", i + 1, skill.name, skill.version);
            println!("     {} | {} downloads", skill.description, skill.downloads);
            println!();
        }

        Ok(())
    }

    async fn list(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: List skills from workspace
        println!("Installed skills:");
        println!("  (This command will be implemented with hot loader)");
        Ok(())
    }

    async fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = ClawHubClient::new()?;

        println!("Updating skill index...");
        let index = client.get_index(0).await?;
        println!("Index updated! Last updated: {}", index.last_updated);
        println!("Total skills: {}", index.skills.len());

        Ok(())
    }

    async fn info(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = ClawHubClient::new()?;
        let results = client.search(name).await?;

        let skill = results.iter()
            .find(|s| s.name == name)
            .ok_or_else(|| format!("Skill '{}' not found", name))?;

        println!("Name: {}", skill.name);
        println!("Version: {}", skill.version);
        println!("Description: {}", skill.description);
        println!("Author: {}", skill.repository);
        println!("Downloads: {}", skill.downloads);
        println!("Source: {}", skill.source_project);
        println!("Tags: {}", skill.tags.join(", "));

        Ok(())
    }
}
```

**Step 2: Add to main command enum**

Modify `src/lib.rs` to add the SkillsCommand:

```rust
pub mod commands;

pub use commands::skills::SkillsCommand;
```

**Step 3: Wire up in main**

Modify `src/main.rs` to add skills command:

```rust
use zeroclaw::SkillsCommand;

// In the Commands enum, add:
#[clap(subcommand)]
Skills(SkillsCommand),
```

**Step 4: Test manually**

```bash
cargo build --release
./target/release/zeroclaw skills search git
./target/release/zeroclaw skills trending --count 5
./target/release/zeroclaw skills update
```

**Step 5: Commit**

```bash
git add src/commands/skills.rs src/lib.rs src/main.rs
git commit -m "feat(cli): add skills management commands"
```

---

## Phase 3: Hot Loader

### Task 3.1: Implement file system watcher for hot loading

**Files:**
- Create: `src/clawhub/hotload.rs`
- Test: `src/clawhub/tests/hotload_tests.rs`

**Step 1: Write the failing test**

Create `src/clawhub/tests/hotload_tests.rs`:

```rust
use zeroclaw::clawhub::hotload::SkillHotLoader;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_hotloader_detects_file_changes() {
    let loader = SkillHotLoader::new("/tmp/test_skills").await.unwrap();

    // Trigger should detect changes
    let detected = loader.check_for_changes().await;
    assert!(detected.is_ok());
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --package zeroclaw clawhub::tests::hotload_tests
```

Expected: "module `hotload` not found"

**Step 3: Implement hot loader**

Create `src/clawhub/hotload.rs`:

```rust
use crate::clawhub::{Result, ClawHubError, SkillSIF};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::time::Duration;

pub struct SkillHotLoader {
    skills_dir: PathBuf,
    skills_cache: Arc<RwLock<std::collections::HashMap<String, SkillSIF>>>,
    _watcher: Option<RecommendedWatcher>,
}

impl SkillHotLoader {
    pub async fn new(skills_dir: &str) -> Result<Self> {
        let skills_dir = PathBuf::from(skills_dir);
        tokio::fs::create_dir_all(&skills_dir).await?;

        let skills_cache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        // TODO: Setup file watcher

        Ok(Self {
            skills_dir,
            skills_cache,
            _watcher: None,
        })
    }

    pub async fn check_for_changes(&self) -> Result<bool> {
        let mut changes = false;

        // Scan directory for new/modified skills
        let mut entries = tokio::fs::read_dir(&self.skills_dir).await?;
        let mut cache = self.skills_cache.write().await;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let skill_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| ClawHubError::InvalidFormat {
                        file: path.display().to_string(),
                        reason: "invalid skill name".to_string(),
                    })?
                    .to_string();

                // Check if skill was modified
                if let Ok(metadata) = tokio::fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Some(cached) = cache.get(&skill_name) {
                            // TODO: Compare modification time
                        } else {
                            // New skill
                            changes = true;
                        }
                    }
                }
            }
        }

        Ok(changes)
    }

    pub async fn reload_all(&self) -> Result<usize> {
        let detector_registry = super::DetectorRegistry::new()
            .register(super::ZeroClawDetector::new());

        let mut cache = self.skills_cache.write().await;
        let mut loaded = 0;

        let mut entries = tokio::fs::read_dir(&self.skills_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                match detector_registry.detect_and_parse(&path).await {
                    Ok(sif) => {
                        let name = sif.metadata.name.clone();
                        cache.insert(name, sif);
                        loaded += 1;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load skill at {}: {}", path.display(), e);
                    }
                }
            }
        }

        tracing::info!("Reloaded {} skills", loaded);
        Ok(loaded)
    }

    pub async fn get_skill(&self, name: &str) -> Option<SkillSIF> {
        let cache = self.skills_cache.read().await;
        cache.get(name).cloned()
    }

    pub async fn list_skills(&self) -> Vec<String> {
        let cache = self.skills_cache.read().await;
        cache.keys().cloned().collect()
    }
}
```

**Step 4: Update module exports**

Update `src/clawhub/mod.rs`:

```rust
pub mod cache;
pub mod detectors;
pub mod error;
pub mod hotload;
pub mod registry;
pub mod sif;
pub mod types;

pub use hotload::SkillHotLoader;
// ... other exports
```

**Step 5: Run tests to verify they pass**

```bash
cargo test --package zeroclaw clawhub::tests::hotload_tests
```

Expected: All tests PASS

**Step 6: Commit**

```bash
git add src/clawhub/hotload.rs src/clawhub/mod.rs src/clawhub/tests/hotload_tests.rs
git commit -m "feat(clawhub): implement hot loader for skill updates"
```

---

## Phase 4: Intelligent Skill Selector

### Task 4.1: Implement complexity analyzer

**Files:**
- Create: `src/skills/selector.rs`
- Create: `src/skills/complexity.rs`

**Step 1: Create complexity analyzer**

Create `src/skills/complexity.rs`:

```rust
/// Analyze task complexity from user message
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    pub fn analyze(message: &str) -> f32 {
        let mut score = 0.0;

        // Length factor (0.0 - 0.3)
        let length_factor = (message.len() as f32 / 1000.0).min(0.3);
        score += length_factor;

        // Keyword analysis
        let complex_keywords = [
            "implement", "design", "architecture", "system",
            "optimize", "refactor", "integrate", "migration",
            "debug", "investigate", "analyze"
        ];

        let simple_keywords = [
            "what", "how", "explain", "show", "list",
            "tell", "status", "help"
        ];

        for keyword in complex_keywords {
            if message.to_lowercase().contains(keyword) {
                score += 0.15;
            }
        }

        for keyword in simple_keywords {
            if message.to_lowercase().contains(keyword) {
                score -= 0.1;
            }
        }

        // Code snippets increase complexity
        if message.contains("```") || message.contains("fn ") || message.contains("class ") {
            score += 0.2;
        }

        // Multiple questions increase complexity
        if message.matches('?').count() > 1 {
            score += 0.1;
        }

        // Clamp between 0.0 and 1.0
        score.max(0.0).min(1.0)
    }

    pub fn get_level(complexity: f32) -> &'static str {
        if complexity < 0.2 { "simple" }
        else if complexity < 0.4 { "moderate" }
        else if complexity < 0.6 { "complex" }
        else { "very_complex" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_task() {
        let complexity = ComplexityAnalyzer::analyze("What time is it?");
        assert!(complexity < 0.3);
    }

    #[test]
    fn test_complex_task() {
        let complexity = ComplexityAnalyzer::analyze(
            "Implement a new authentication system with OAuth2 integration \
             and refactor the existing session management"
        );
        assert!(complexity > 0.5);
    }
}
```

**Step 2: Create selector**

Create `src/skills/selector.rs`:

```rust
use crate::clawhub::{SkillSIF, ClawHubClient};
use crate::skills::complexity::ComplexityAnalyzer;
use std::collections::HashMap;

pub struct SkillSelector {
    complexity_threshold: f32,
    skills: HashMap<String, SkillSIF>,
}

impl SkillSelector {
    pub fn new(complexity_threshold: f32) -> Self {
        Self {
            complexity_threshold,
            skills: HashMap::new(),
        }
    }

    pub async fn select_skills_for_task(
        &self,
        task: &str,
        disclosure_level: DisclosureLevel,
    ) -> Vec<SkillInfo> {
        let complexity = ComplexityAnalyzer::analyze(task);

        // Simple tasks don't need skills
        if complexity < self.complexity_threshold {
            tracing::debug!("Task complexity {:.2} below threshold {:.2}, no skills needed",
                complexity, self.complexity_threshold);
            return Vec::new();
        }

        // Get relevant skills based on task
        let relevant = self.find_relevant_skills(task);

        // Apply progressive disclosure
        relevant.into_iter()
            .map(|skill| self.apply_disclosure(skill, disclosure_level))
            .collect()
    }

    fn find_relevant_skills(&self, task: &str) -> Vec<SkillInfo> {
        let task_lower = task.to_lowercase();
        let mut relevant = Vec::new();

        for (name, skill) in &self.skills {
            // Check name match
            if task_lower.contains(&name.to_lowercase()) {
                relevant.push(SkillInfo::from_sif(name.clone(), skill.clone()));
                continue;
            }

            // Check tags match
            for tag in &skill.metadata.tags {
                if task_lower.contains(&tag.to_lowercase()) {
                    relevant.push(SkillInfo::from_sif(name.clone(), skill.clone()));
                    break;
                }
            }

            // Check description match
            if skill.metadata.description.to_lowercase().contains(&task_lower) {
                relevant.push(SkillInfo::from_sif(name.clone(), skill.clone()));
            }
        }

        relevant
    }

    fn apply_disclosure(&self, mut skill: SkillInfo, level: DisclosureLevel) -> SkillInfo {
        match level {
            DisclosureLevel::NameOnly => {
                skill.description = None;
                skill.tags = None;
                skill.content = None;
            }
            DisclosureLevel::Description => {
                skill.tags = None;
                skill.content = None;
            }
            DisclosureLevel::Tags => {
                skill.content = None;
            }
            DisclosureLevel::Full => {
                // Show everything
            }
        }

        skill
    }
}

#[derive(Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub content: Option<String>,
}

impl SkillInfo {
    fn from_sif(name: String, sif: SkillSIF) -> Self {
        Self {
            name,
            description: Some(sif.metadata.description),
            tags: Some(sif.metadata.tags),
            content: Some(sif.content),
        }
    }
}

#[derive(Clone, Copy)]
pub enum DisclosureLevel {
    NameOnly,
    Description,
    Tags,
    Full,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_below_threshold() {
        let selector = SkillSelector::new(0.5);
        let task = "What is ZeroClaw?";

        // Should return empty (simple task)
        // assert!(!result.is_empty()); // Will be tested with actual implementation
    }
}
```

**Step 3: Update skills module**

Modify `src/skills/mod.rs` to export selector:

```rust
pub mod complexity;
pub mod selector;

pub use selector::{SkillSelector, SkillInfo, DisclosureLevel};
```

**Step 4: Commit**

```bash
git add src/skills/selector.rs src/skills/complexity.rs src/skills/mod.rs
git commit -m "feat(skills): implement intelligent skill selector with complexity analysis"
```

---

## Phase 5: Telemetry System

### Task 5.1: Create telemetry storage schema

**Files:**
- Create: `src/telemetry/mod.rs`
- Create: `src/telemetry/storage.rs`

**Step 1: Implement storage**

Create `src/telemetry/storage.rs`:

```rust
use crate::clawhub::{Result, ClawHubError};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::time::SystemTime;

pub struct TelemetryStorage {
    conn: Connection,
}

impl TelemetryStorage {
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path)
            .map_err(|e| ClawHubError::DatabaseError(e.to_string()))?;

        let storage = Self { conn };
        storage.init_schema().await?;
        Ok(storage)
    }

    async fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS skill_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_name TEXT NOT NULL,
                task_complexity REAL,
                task_type TEXT,
                success BOOLEAN,
                execution_time_ms INTEGER,
                tokens_used INTEGER,
                user_satisfaction REAL,
                selected_by TEXT,
                context_fingerprint TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| ClawHubError::DatabaseError(e.to_string()))?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_skill_name ON skill_usage(skill_name)",
            [],
        ).map_err(|e| ClawHubError::DatabaseError(e.to_string()))?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON skill_usage(timestamp)",
            [],
        ).map_err(|e| ClawHubError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn record_skill_usage(
        &self,
        skill_name: &str,
        task_complexity: f32,
        task_type: &str,
        success: bool,
        execution_time_ms: u64,
        tokens_used: u32,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO skill_usage (skill_name, task_complexity, task_type, success, execution_time_ms, tokens_used)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![skill_name, task_complexity, task_type, success as i32, execution_time_ms as i64, tokens_used as i64],
        ).map_err(|e| ClawHubError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
```

**Step 2: Create telemetry module**

Create `src/telemetry/mod.rs`:

```rust
pub mod storage;

pub use storage::TelemetryStorage;
```

**Step 3: Commit**

```bash
git add src/telemetry/mod.rs src/telemetry/storage.rs
git commit -m "feat(telemetry): add SQLite storage backend"
```

---

### Task 5.2: Implement usage collector

**Files:**
- Create: `src/telemetry/collector.rs`

**Step 1: Implement collector**

Create `src/telemetry/collector.rs`:

```rust
use crate::telemetry::storage::TelemetryStorage;
use std::sync::Arc;
use std::time::Instant;

pub struct TelemetryCollector {
    storage: Arc<TelemetryStorage>,
    enabled: bool,
}

impl TelemetryCollector {
    pub fn new(storage: Arc<TelemetryStorage>, enabled: bool) -> Self {
        Self { storage, enabled }
    }

    pub fn record_skill_usage(
        &self,
        skill_name: &str,
        task_complexity: f32,
        task_type: &str,
        f: impl FnOnce() -> Result<(bool, u64, u32), Box<dyn std::error::Error>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            // Still execute the function, just don't record
            f()?;
            return Ok(());
        }

        let start = Instant::now();
        let result = f()?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        self.storage.record_skill_usage(
            skill_name,
            task_complexity,
            task_type,
            result.0,
            execution_time_ms,
            result.2,
        )?;

        Ok(())
    }
}
```

**Step 2: Update module**

Update `src/telemetry/mod.rs`:

```rust
pub mod collector;
pub mod storage;

pub use collector::TelemetryCollector;
pub use storage::TelemetryStorage;
```

**Step 3: Commit**

```bash
git add src/telemetry/collector.rs src/telemetry/mod.rs
git commit -m "feat(telemetry): add usage collector"
```

---

## Phase 6: Smart Builtin Recommendation

### Task 6.1: Implement builtin recommendation engine

**Files:**
- Create: `src/telemetry/builtin.rs`

**Step 1: Implement builtin detector**

Create `src/telemetry/builtin.rs`:

```rust
use crate::telemetry::storage::TelemetryStorage;
use crate::clawhub::{Result, ClawHubError};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct BuiltinCandidate {
    pub name: String,
    pub call_count_30d: u32,
    pub success_rate: f32,
    pub avg_latency_ms: u64,
}

pub struct BuiltinRecommender {
    storage: Arc<TelemetryStorage>,
}

impl BuiltinRecommender {
    pub fn new(storage: Arc<TelemetryStorage>) -> Self {
        Self { storage }
    }

    pub async fn analyze_candidates(&self) -> Result<Vec<BuiltinCandidate>> {
        // Query database for frequently used skills
        let mut candidates = Vec::new();

        // TODO: Implement SQL query to find skills with:
        // - >100 calls in 30 days
        // - >95% success rate
        // - High external overhead

        Ok(candidates)
    }

    pub async fn should_recommend(&self, candidate: &BuiltinCandidate) -> bool {
        candidate.call_count_30d >= 100
            && candidate.success_rate >= 0.95
            && candidate.avg_latency_ms > 100
    }

    pub async fn generate_recommendation(
        &self,
        candidate: &BuiltinCandidate,
    ) -> Result<String> {
        Ok(format!(
            "🚀 Builtin Recommendation: {}\n\
             - 30-day calls: {}\n\
             - Success rate: {:.1}%\n\
             - Avg latency: {}ms",
            candidate.name,
            candidate.call_count_30d,
            candidate.success_rate * 100.0,
            candidate.avg_latency_ms
        ))
    }
}
```

**Step 2: Update module**

Update `src/telemetry/mod.rs`:

```rust
pub mod builtin;
pub mod collector;
pub mod storage;

pub use builtin::{BuiltinRecommender, BuiltinCandidate};
pub use collector::TelemetryCollector;
pub use storage::TelemetryStorage;
```

**Step 3: Commit**

```bash
git add src/telemetry/builtin.rs src/telemetry/mod.rs
git commit -m "feat(telemetry): add builtin recommendation engine"
```

---

## Final Integration Tasks

### Task 7.1: Update agent to use skill selector

**Files:**
- Modify: `src/agent/mod.rs`

**Step 1: Integrate selector into agent loop**

Modify agent to use `SkillSelector` when building prompts:

```rust
// In agent orchestration
let selector = SkillSelector::new(config.skills_complexity_threshold);
let skills = selector.select_skills_for_task(&user_message, DisclosureLevel::Description).await;

// Add skills to prompt
for skill in skills {
    prompt.push_str(&format!("\n## Skill: {}\n", skill.name));
    if let Some(desc) = skill.description {
        prompt.push_str(&format!("{}\n", desc));
    }
}
```

**Step 2: Commit**

```bash
git add src/agent/mod.rs
git commit -m "feat(agent): integrate intelligent skill selection"
```

---

### Task 7.2: Add telemetry hooks throughout codebase

**Files:**
- Modify: `src/agent/mod.rs`
- Modify: `src/tools/mod.rs`

**Step 1: Add telemetry to tool execution**

```rust
// In tool execution
telemetry.record_tool_usage(
    tool_name,
    || execute_tool(tool)
).await;
```

**Step 2: Commit**

```bash
git add src/agent/mod.rs src/tools/mod.rs
git commit -m "feat(telemetry): add instrumentation hooks"
```

---

## Testing & Documentation

### Task 8.1: Add integration tests

**Files:**
- Create: `tests/clawhub_integration.rs`

**Step 1: Create integration test suite**

```rust
use zeroclaw::clawhub::*;

#[tokio::test]
async fn test_full_skill_installation_workflow() {
    // Test: search -> download -> convert -> load
}

#[tokio::test]
async fn test_hot_reload_workflow() {
    // Test: add skill -> detect -> reload
}
```

**Step 2: Commit**

```bash
git add tests/clawhub_integration.rs
git commit -m "test(clawhub): add integration test suite"
```

---

### Task 8.2: Update documentation

**Files:**
- Create: `docs/clawhub-usage.md`
- Update: `README.md`

**Step 1: Create usage guide**

Create comprehensive guide covering:
- Installing skills from ClawHub
- Configuring skill selection
- Using telemetry commands
- Interpreting builtin recommendations

**Step 2: Update README**

Add ClawHub integration section with quick start

**Step 3: Commit**

```bash
git add docs/clawhub-usage.md README.md
git commit -m "docs: add ClawHub integration usage guide"
```

---

## Success Criteria

Run validation:

```bash
# Format check
cargo fmt --all -- --check

# Lint check
cargo clippy --all-targets -- -D warnings

# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Build release
cargo build --release

# Manual testing
./target/release/zeroclaw skills search git
./target/release/zeroclaw skills trending
./target/release/zeroclaw telemetry skills --top 10
```

**Expected outcomes:**
- ✅ All tests pass
- ✅ Zero clippy warnings
- ✅ Can search and install skills from ClawHub
- ✅ Skills hot-load without restart
- ✅ Telemetry tracks usage
- ✅ Selector intelligently chooses skills
- ✅ Builtin recommendations work

---

## Rollback Plan

If issues arise:
1. Disable ClawHub: `[clawhub.enabled = false]`
2. Remove `src/clawhub/` module
3. Revert CLI changes
4. Keep existing `skills/` untouched

---

## Next Steps After Implementation

1. **Performance Testing** - Load test with 100+ skills
2. **Security Audit** - Review path validation and sandboxing
3. **User Testing** - Get feedback on selection quality
4. **Documentation Review** - Ensure all features are documented
5. **Community Validation** - Test with real ClawHub skills
