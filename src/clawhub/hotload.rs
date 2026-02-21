//! Hot loader for skill files
//!
//! Monitors the skills directory for changes and automatically reloads skills
//! without requiring a restart of the ZeroClaw daemon.

use crate::skills::Skill;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Debounce delay for file system events (ms)
const DEBOUNCE_DELAY_MS: u64 = 300;

/// Hot loader for skill files
///
/// Monitors the skills directory and automatically reloads skills when
/// SKILL.toml or SKILL.md files change.
#[derive(Clone)]
pub struct SkillHotLoader {
    /// Path to the skills directory
    skills_dir: PathBuf,
    /// Cache of loaded skills (name -> Skill)
    skills_cache: Arc<RwLock<HashMap<String, Skill>>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

impl SkillHotLoader {
    /// Create a new hot loader for the given skills directory
    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            skills_cache: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get the skills directory path
    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    /// Reload all skills from disk
    pub async fn reload_all(&self) -> Result<()> {
        debug!("Reloading all skills from {}", self.skills_dir.display());

        // The skills_dir is the workspace directory containing the skills subdirectory
        let skills = crate::skills::load_skills(&self.skills_dir);
        let mut cache = self.skills_cache.write().await;

        // Clear and rebuild cache
        cache.clear();
        for skill in skills {
            cache.insert(skill.name.clone(), skill);
        }

        debug!("Reloaded {} skills", cache.len());
        Ok(())
    }

    /// Create a hot loader from a workspace directory
    pub fn from_workspace(workspace_dir: PathBuf) -> Self {
        Self::new(workspace_dir)
    }

    /// Get a skill by name
    pub async fn get_skill(&self, name: &str) -> Option<Skill> {
        let cache = self.skills_cache.read().await;
        cache.get(name).cloned()
    }

    /// List all loaded skill names
    pub async fn list_skills(&self) -> Vec<String> {
        let cache = self.skills_cache.read().await;
        let mut names: Vec<String> = cache.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get all skills
    pub async fn all_skills(&self) -> Vec<Skill> {
        let cache = self.skills_cache.read().await;
        cache.values().cloned().collect()
    }

    /// Check if the hot loader is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Start watching for file changes
    ///
    /// This spawns a background task that monitors the skills directory
    /// and reloads skills when files change.
    #[cfg(feature = "hotload")]
    pub async fn start_watching(&self) -> Result<()> {
        use notify::{RecursiveMode, Watcher};
        use std::sync::mpsc::channel;

        // Check if already running
        {
            let mut running = self.running.write().await;
            if *running {
                warn!("Hot loader is already running");
                return Ok(());
            }
            *running = true;
        }

        // Initial load
        self.reload_all().await?;

        let skills_dir = self.skills_dir.clone();
        let skills_cache = self.skills_cache.clone();
        let running_flag = self.running.clone();

        // Spawn watcher task
        tokio::spawn(async move {
            if let Err(e) = Self::watch_loop(skills_dir, skills_cache, running_flag).await {
                warn!("Hot loader watch loop ended with error: {e}");
            }
        });

        info!("Started hot loader for {}", self.skills_dir.display());
        Ok(())
    }

    /// Stop watching for file changes
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Stopped hot loader");
    }

    /// Internal watch loop
    #[cfg(feature = "hotload")]
    async fn watch_loop(
        skills_dir: PathBuf,
        skills_cache: Arc<RwLock<HashMap<String, Skill>>>,
        running_flag: Arc<RwLock<bool>>,
    ) -> Result<()> {
        use notify::{RecursiveMode, Watcher};
        use std::sync::mpsc::channel;
        use std::time::Instant;

        let (tx, rx) = channel();

        let mut watcher = notify::debounce_full::Debouncer::new_with_delay(
            Duration::from_millis(DEBOUNCE_DELAY_MS),
            move |res: Result<Vec<notify::Event>, notify::Error>| {
                if let Ok(events) = res {
                    for event in events {
                        let _ = tx.send(event);
                    }
                }
            },
        )?;

        watcher.watch(&skills_dir, RecursiveMode::Recursive)?;

        info!("Watching {} for changes", skills_dir.display());

        while *running_flag.read().await {
            // Check for events with timeout
            if let Ok(event) = rx.recv_timeout(Duration::from_secs(1)) {
                debug!("File system event: {:?}", event);

                // Reload on any relevant event
                if Self::should_reload(&event, &skills_dir) {
                    if let Err(e) = Self::do_reload(&skills_dir, &skills_cache).await {
                        warn!("Failed to reload skills: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if an event should trigger a reload
    #[cfg(feature = "hotload")]
    fn should_reload(event: &notify::Event, skills_dir: &Path) -> bool {
        // Ignore events without paths
        if event.paths.is_empty() {
            return false;
        }

        for path in &event.paths {
            // Check if the path is within the skills directory
            if let Ok(rel_path) = path.strip_prefix(skills_dir) {
                // Security: Verify no path traversal components in relative path
                // This prevents symlink-based escapes outside the skills directory
                if rel_path
                    .components()
                    .any(|c| c == std::path::Component::ParentDir)
                {
                    warn!(
                        "Path traversal attempt detected: {} contains '..' components",
                        path.display()
                    );
                    continue;
                }

                // Check if it's a SKILL.toml or SKILL.md file
                if let Some(file_name) = path.file_name() {
                    let name = file_name.to_string_lossy();
                    if name == "SKILL.toml" || name == "SKILL.md" {
                        return true;
                    }
                }

                // Also reload if a directory is created/removed
                if path.is_dir() {
                    return true;
                }
            }
        }

        false
    }

    /// Perform a reload
    async fn do_reload(
        skills_dir: &Path,
        skills_cache: &Arc<RwLock<HashMap<String, Skill>>>,
    ) -> Result<()> {
        debug!("Reloading skills due to file system event");
        let skills = crate::skills::load_skills(skills_dir);
        let mut cache = skills_cache.write().await;

        cache.clear();
        for skill in skills {
            cache.insert(skill.name.clone(), skill);
        }

        info!("Reloaded {} skills", cache.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hotloader_new() {
        let temp_dir = TempDir::new().unwrap();
        let loader = SkillHotLoader::new(temp_dir.path().to_path_buf());

        assert_eq!(loader.skills_dir(), temp_dir.path());
    }

    #[tokio::test]
    async fn test_reload_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let loader = SkillHotLoader::new(temp_dir.path().to_path_buf());

        loader.reload_all().await.unwrap();

        let skills = loader.list_skills().await;
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_reload_with_skills() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();
        let skills_dir = workspace_dir.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        // Create a test skill
        let skill_dir = skills_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.toml"),
            r#"
[skill]
name = "test-skill"
description = "A test skill"
version = "1.0.0"
"#,
        )
        .unwrap();

        let loader = SkillHotLoader::from_workspace(workspace_dir.to_path_buf());
        loader.reload_all().await.unwrap();

        let skills = loader.list_skills().await;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], "test-skill");

        let skill = loader.get_skill("test-skill").await;
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().description, "A test skill");
    }

    #[tokio::test]
    async fn test_get_skill_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let loader = SkillHotLoader::new(temp_dir.path().to_path_buf());

        let skill = loader.get_skill("nonexistent").await;
        assert!(skill.is_none());
    }

    #[tokio::test]
    async fn test_all_skills() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();
        let skills_dir = workspace_dir.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        // Create multiple skills
        for name in ["alpha", "beta", "gamma"] {
            let skill_dir = skills_dir.join(name);
            fs::create_dir_all(&skill_dir).unwrap();
            fs::write(
                skill_dir.join("SKILL.md"),
                format!("# {}\nSkill {name} description.\n", name),
            )
            .unwrap();
        }

        let loader = SkillHotLoader::from_workspace(workspace_dir.to_path_buf());
        loader.reload_all().await.unwrap();

        let all_skills = loader.all_skills().await;
        assert_eq!(all_skills.len(), 3);
    }

    #[tokio::test]
    async fn test_list_skills_sorted() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();
        let skills_dir = workspace_dir.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        // Create skills in non-alphabetical order
        for name in ["zebra", "alpha", "beta"] {
            let skill_dir = skills_dir.join(name);
            fs::create_dir_all(&skill_dir).unwrap();
            fs::write(
                skill_dir.join("SKILL.md"),
                format!("# {}\nSkill {name}.\n", name),
            )
            .unwrap();
        }

        let loader = SkillHotLoader::from_workspace(workspace_dir.to_path_buf());
        loader.reload_all().await.unwrap();

        let skills = loader.list_skills().await;
        assert_eq!(skills, vec!["alpha", "beta", "zebra"]);
    }

    #[tokio::test]
    async fn test_reload_updates_cache() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();
        let skills_dir = workspace_dir.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        // Create initial skill
        let skill_dir = skills_dir.join("mutable");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.toml"),
            r#"
[skill]
name = "mutable"
description = "Original description"
version = "1.0.0"
"#,
        )
        .unwrap();

        let loader = SkillHotLoader::from_workspace(workspace_dir.to_path_buf());
        loader.reload_all().await.unwrap();

        let skill = loader.get_skill("mutable").await;
        assert_eq!(skill.unwrap().description, "Original description");

        // Modify the skill
        fs::write(
            skill_dir.join("SKILL.toml"),
            r#"
[skill]
name = "mutable"
description = "Updated description"
version = "1.0.0"
"#,
        )
        .unwrap();

        loader.reload_all().await.unwrap();

        let skill = loader.get_skill("mutable").await;
        assert_eq!(skill.unwrap().description, "Updated description");
    }

    #[tokio::test]
    async fn test_is_running() {
        let temp_dir = TempDir::new().unwrap();
        let loader = SkillHotLoader::new(temp_dir.path().to_path_buf());

        assert!(!loader.is_running().await);

        // Note: We can't test start_watching without the hotload feature
        // and it's complex to test in unit tests, so we just verify the flag
    }

    #[test]
    fn test_debounce_constant() {
        assert_eq!(DEBOUNCE_DELAY_MS, 300);
    }
}
