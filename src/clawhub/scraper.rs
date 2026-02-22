//! ClawHub.ai web scraper
//!
//! This module provides functionality to scrape skill data from clawhub.ai

use crate::clawhub::{ClawHubError, ClawHubIndex, Result, SkillIndexEntry};
use std::process::Command;
use std::time::Duration;

const CLAWHUB_AI_BASE: &str = "https://clawhub.ai";

/// Scraper for clawhub.ai website
pub struct ClawHubScraper {
    client: reqwest::blocking::Client,
    use_cli: bool,
}

impl ClawHubScraper {
    /// Create a new scraper instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ZeroClaw/0.1.0 (+https://github.com/zeroclaw-labs/zeroclaw)")
                .build()?,
            // Don't cache CLI availability - check at runtime
            use_cli: false,
        })
    }

    /// Check if clawhub CLI is available
    fn is_clawhub_cli_available(&self) -> bool {
        let cmd = if cfg!(windows) { "clawhub.cmd" } else { "clawhub" };
        Command::new(cmd)
            .arg("--cli-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Fetch skills from the skills page with sorting
    pub fn fetch_skills(&self, sort: Option<&str>, limit: Option<usize>) -> Result<ClawHubIndex> {
        // Try CLI first, fall back to web scraping
        match self.fetch_skills_via_cli(sort, limit) {
            Ok(index) => Ok(index),
            Err(e) => {
                // If CLI failed, try web scraping
                self.fetch_skills_via_web(sort, limit).map_err(|_web_err| {
                    // Return original CLI error if web scraping also fails
                    e
                })
            }
        }
    }

    /// Fetch skills using clawhub CLI
    fn fetch_skills_via_cli(&self, sort: Option<&str>, limit: Option<usize>) -> Result<ClawHubIndex> {
        let limit = limit.unwrap_or(50);
        let sort_by = sort.unwrap_or("downloads");

        // On Windows, clawhub is installed via npm as a batch file
        let cmd = if cfg!(windows) {
            "clawhub.cmd"
        } else {
            "clawhub"
        };

        let output = Command::new(cmd)
            .arg("explore")
            .arg("--json")
            .arg("--limit")
            .arg(limit.to_string())
            .arg("--sort")
            .arg(sort_by)
            .output()
            .map_err(|e| ClawHubError::InvalidFormat {
                file: "clawhub CLI".to_string(),
                reason: format!("Failed to execute: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ClawHubError::InvalidFormat {
                file: "clawhub CLI".to_string(),
                reason: format!("CLI error: {}", stderr),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output from clawhub CLI
        #[derive(serde::Deserialize)]
        struct ClawHubResponse {
            items: Vec<ClawHubItem>,
        }

        #[derive(serde::Deserialize)]
        struct ClawHubItem {
            slug: String,
            #[serde(default)]
            display_name: Option<String>,
            #[serde(default)]
            summary: Option<String>,
            stats: ClawHubStats,
            #[serde(default)]
            tags: std::collections::HashMap<String, String>,
        }

        #[derive(serde::Deserialize)]
        struct ClawHubStats {
            #[serde(default)]
            downloads: u64,
            #[serde(default)]
            stars: u64,
            #[serde(default)]
            versions: u64,
        }

        let response: ClawHubResponse = serde_json::from_str(&stdout)
            .map_err(|e| ClawHubError::InvalidFormat {
                file: "clawhub CLI output".to_string(),
                reason: format!("Failed to parse JSON: {}", e),
            })?;

        let skills = response.items.into_iter().map(|item| SkillIndexEntry {
            name: item.display_name.unwrap_or_else(|| item.slug.clone()),
            description: item.summary.unwrap_or_else(|| {
                format!("Skill from clawhub.ai/{}", item.slug)
            }),
            version: item.tags.get("latest").cloned().unwrap_or_else(|| {
                item.tags.values().next().cloned().unwrap_or_else(|| "latest".to_string())
            }),
            downloads: item.stats.downloads,
            source_project: "clawhub".to_string(),
            repository: format!("{}/{}", CLAWHUB_AI_BASE, item.slug),
            tags: item.tags.into_keys().collect(),
        }).collect();

        Ok(ClawHubIndex {
            skills,
            last_updated: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Fetch skills using web scraping (fallback)
    fn fetch_skills_via_web(&self, _sort: Option<&str>, _limit: Option<usize>) -> Result<ClawHubIndex> {
        // Note: clawhub.ai is a SPA, so simple HTML scraping won't work
        // This is a fallback that returns a helpful error message
        Err(ClawHubError::DependencyMissing {
            dependency: "clawhub CLI".to_string(),
        })
    }

    /// Fetch popular/trending skills (sorted by downloads)
    pub fn fetch_popular_skills(&self, limit: Option<usize>) -> Result<ClawHubIndex> {
        self.fetch_skills(Some("downloads"), limit)
    }

    /// Search skills by query
    pub fn search_skills(&self, query: &str) -> Result<Vec<SkillIndexEntry>> {
        let cmd = if cfg!(windows) { "clawhub.cmd" } else { "clawhub" };

        let output = Command::new(cmd)
            .arg("search")
            .arg(query)
            .arg("--json")
            .arg("--limit")
            .arg("50")
            .output();

        match output {
            Ok(result) if result.status.success() => {
                let stdout = String::from_utf8_lossy(&result.stdout);

                #[derive(serde::Deserialize)]
                struct ClawHubResponse {
                    items: Vec<ClawHubItem>,
                }

                #[derive(serde::Deserialize)]
                struct ClawHubItem {
                    slug: String,
                    #[serde(default)]
                    display_name: Option<String>,
                    #[serde(default)]
                    summary: Option<String>,
                    stats: ClawHubStats,
                }

                #[derive(serde::Deserialize)]
                struct ClawHubStats {
                    #[serde(default)]
                    downloads: u64,
                }

                if let Ok(response) = serde_json::from_str::<ClawHubResponse>(&stdout) {
                    let skills = response.items.into_iter().map(|item| SkillIndexEntry {
                        name: item.display_name.unwrap_or(item.slug.clone()),
                        description: item.summary.unwrap_or_default(),
                        version: "latest".to_string(),
                        downloads: item.stats.downloads,
                        source_project: "clawhub".to_string(),
                        repository: format!("{}/{}", CLAWHUB_AI_BASE, item.slug),
                        tags: vec![],
                    }).collect();
                    Ok(skills)
                } else {
                    Ok(vec![])
                }
            }
            _ => {
                // Fallback to local search if CLI fails
                let index = self.fetch_popular_skills(Some(200))?;
                let query_lower = query.to_lowercase();
                let results: Vec<SkillIndexEntry> = index.skills.iter().filter(|skill| {
                    skill.name.to_lowercase().contains(&query_lower)
                        || skill.description.to_lowercase().contains(&query_lower)
                        || skill
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(&query_lower))
                        || skill.repository.to_lowercase().contains(&query_lower)
                }).cloned().collect();
                Ok(results)
            }
        }
    }

    /// Get skill by name/slug
    pub fn get_skill(&self, name: &str) -> Result<Option<SkillIndexEntry>> {
        let index = self.fetch_skills(Some("downloads"), Some(200))?;

        Ok(index.skills.iter().find(|s| {
            s.name.eq_ignore_ascii_case(name)
                || s.repository.ends_with(&format!("/{}", name))
                || s.repository.ends_with(&format!("/{}", name.replace('-', "_")))
        }).cloned())
    }
}

impl Default for ClawHubScraper {
    fn default() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ZeroClaw/0.1.0")
                .build()
                .unwrap(),
            // CLI availability is checked at runtime
            use_cli: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scraper_default() {
        let scraper = ClawHubScraper::default();
        assert_eq!(scraper.client.user_agent().unwrap(), "ZeroClaw/0.1.0");
    }

    #[test]
    fn test_scraper_no_cli_by_default() {
        let scraper = ClawHubScraper::default();
        // Most test environments won't have clawhub CLI installed
        assert!(!scraper.use_cli);
    }
}

