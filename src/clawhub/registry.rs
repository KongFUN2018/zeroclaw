use crate::clawhub::{
    cache::ClawHubCache,
    types::{ClawHubIndex, SkillIndexEntry},
    Result,
};
use std::time::Duration;

/// Default ClawHub API endpoint
const DEFAULT_API_BASE: &str = "https://api.clawhub.dev";

/// HTTP client for ClawHub registry API
pub struct ClawHubClient {
    client: reqwest::blocking::Client,
    api_base: String,
    cache: ClawHubCache,
}

impl ClawHubClient {
    /// Create a new ClawHub client with default settings
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?,
            api_base: DEFAULT_API_BASE.to_string(),
            cache: ClawHubCache::new()?,
        })
    }

    /// Create a new client with custom API base URL
    pub fn with_api_base(mut self, api_base: String) -> Self {
        self.api_base = api_base;
        self
    }

    /// Create a new client with custom cache
    pub fn with_cache(mut self, cache: ClawHubCache) -> Self {
        self.cache = cache;
        self
    }

    /// Fetch the complete index from ClawHub
    pub fn fetch_index(&self) -> Result<ClawHubIndex> {
        let url = format!("{}/index.json", self.api_base);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(crate::clawhub::ClawHubError::InvalidFormat {
                file: "API response".to_string(),
                reason: format!("HTTP status: {}", response.status()),
            });
        }

        let index: ClawHubIndex = response.json()?;
        Ok(index)
    }

    /// Get the index, using cache if available and fresh
    pub fn get_index(&self) -> Result<ClawHubIndex> {
        // Try cache first
        if let Some(cached) = self.cache.get()? {
            return Ok(cached);
        }

        // Fetch from API
        let index = self.fetch_index()?;

        // Update cache
        self.cache.put(&index)?;

        Ok(index)
    }

    /// Force refresh the index from the API
    pub fn refresh_index(&self) -> Result<ClawHubIndex> {
        let index = self.fetch_index()?;
        self.cache.put(&index)?;
        Ok(index)
    }

    /// Search for skills by query string
    pub fn search(&self, query: &str) -> Result<Vec<SkillIndexEntry>> {
        let index = self.get_index()?;
        let query_lower = query.to_lowercase();

        let results: Vec<SkillIndexEntry> = index
            .skills
            .iter()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&query_lower)
                    || skill.description.to_lowercase().contains(&query_lower)
                    || skill
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// Get trending skills (sorted by downloads)
    pub fn get_trending(&self, limit: Option<usize>) -> Result<Vec<SkillIndexEntry>> {
        let index = self.get_index()?;

        let mut skills = index.skills;
        skills.sort_by(|a, b| b.downloads.cmp(&a.downloads));

        if let Some(limit) = limit {
            skills.truncate(limit);
        }

        Ok(skills)
    }

    /// Get skill by name
    pub fn get_skill(&self, name: &str) -> Result<Option<SkillIndexEntry>> {
        let index = self.get_index()?;

        Ok(index.skills.iter().find(|s| s.name == name).cloned())
    }

    /// Check if cache is expired
    pub fn is_cache_expired(&self) -> bool {
        self.cache.is_expired()
    }
}

impl Default for ClawHubClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback if default creation fails
            Self {
                client: reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .unwrap(),
                api_base: DEFAULT_API_BASE.to_string(),
                cache: ClawHubCache::default(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_default() {
        let client = ClawHubClient::default();
        assert_eq!(client.api_base, DEFAULT_API_BASE);
    }

    #[test]
    fn test_client_with_api_base() {
        let client = ClawHubClient::default().with_api_base("https://custom.api".to_string());
        assert_eq!(client.api_base, "https://custom.api");
    }

    #[test]
    fn test_search_empty_query() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

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

        cache.put(&index).unwrap();

        let client = ClawHubClient::default().with_cache(cache);
        let results = client.search("").unwrap();

        // Empty query should return all results
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![
                SkillIndexEntry {
                    name: "test-skill".to_string(),
                    description: "A test skill".to_string(),
                    version: "1.0.0".to_string(),
                    downloads: 100,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/repo".to_string(),
                    tags: vec!["test".to_string()],
                },
                SkillIndexEntry {
                    name: "other-skill".to_string(),
                    description: "Another skill".to_string(),
                    version: "1.0.0".to_string(),
                    downloads: 50,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/repo2".to_string(),
                    tags: vec!["other".to_string()],
                },
            ],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        cache.put(&index).unwrap();

        let client = ClawHubClient::default().with_cache(cache);
        let results = client.search("test").unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-skill");
    }

    #[test]
    fn test_search_by_tag() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![SkillIndexEntry {
                name: "test-skill".to_string(),
                description: "A test skill".to_string(),
                version: "1.0.0".to_string(),
                downloads: 100,
                source_project: "zeroclaw".to_string(),
                repository: "https://github.com/test/repo".to_string(),
                tags: vec!["automation".to_string(), "test".to_string()],
            }],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        cache.put(&index).unwrap();

        let client = ClawHubClient::default().with_cache(cache);
        let results = client.search("automation").unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-skill");
    }

    #[test]
    fn test_get_trending() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![
                SkillIndexEntry {
                    name: "popular-skill".to_string(),
                    description: "A popular skill".to_string(),
                    version: "1.0.0".to_string(),
                    downloads: 1000,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/repo".to_string(),
                    tags: vec!["popular".to_string()],
                },
                SkillIndexEntry {
                    name: "less-popular".to_string(),
                    description: "Less popular".to_string(),
                    version: "1.0.0".to_string(),
                    downloads: 100,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/repo2".to_string(),
                    tags: vec!["other".to_string()],
                },
            ],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        cache.put(&index).unwrap();

        let client = ClawHubClient::default().with_cache(cache);
        let trending = client.get_trending(None).unwrap();

        assert_eq!(trending.len(), 2);
        assert_eq!(trending[0].name, "popular-skill");
        assert_eq!(trending[0].downloads, 1000);
    }

    #[test]
    fn test_get_trending_with_limit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![
                SkillIndexEntry {
                    name: "skill1".to_string(),
                    description: "Skill 1".to_string(),
                    version: "1.0.0".to_string(),
                    downloads: 1000,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/repo".to_string(),
                    tags: vec![],
                },
                SkillIndexEntry {
                    name: "skill2".to_string(),
                    description: "Skill 2".to_string(),
                    version: "1.0.0".to_string(),
                    downloads: 500,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/repo2".to_string(),
                    tags: vec![],
                },
            ],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        cache.put(&index).unwrap();

        let client = ClawHubClient::default().with_cache(cache);
        let trending = client.get_trending(Some(1)).unwrap();

        assert_eq!(trending.len(), 1);
        assert_eq!(trending[0].name, "skill1");
    }

    #[test]
    fn test_get_skill() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

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

        cache.put(&index).unwrap();

        let client = ClawHubClient::default().with_cache(cache);
        let skill = client.get_skill("test-skill").unwrap();

        assert!(skill.is_some());
        assert_eq!(skill.unwrap().name, "test-skill");
    }

    #[test]
    fn test_get_skill_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        cache.put(&index).unwrap();

        let client = ClawHubClient::default().with_cache(cache);
        let skill = client.get_skill("non-existent").unwrap();

        assert!(skill.is_none());
    }
}
