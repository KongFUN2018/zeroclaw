use crate::clawhub::{types::ClawHubIndex, Result};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Default cache TTL: 1 hour
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Local cache for ClawHub index with TTL support
pub struct ClawHubCache {
    cache_dir: PathBuf,
    cache_ttl: Duration,
}

impl ClawHubCache {
    /// Create a new cache instance with default settings
    pub fn new() -> Result<Self> {
        let cache_dir = directories::ProjectDirs::from("dev", "zeroclaw", "clawhub")
            .map(|proj| proj.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".cache/zeroclaw/clawhub"));

        Ok(Self {
            cache_dir,
            cache_ttl: DEFAULT_CACHE_TTL,
        })
    }

    /// Create a new cache instance with custom cache directory
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }

    /// Create a new cache instance with custom TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Get the cached index if available and not expired
    pub fn get(&self) -> Result<Option<ClawHubIndex>> {
        let cache_file = self.cache_dir.join("index.json");

        if !cache_file.exists() {
            return Ok(None);
        }

        // Check if cache is expired
        if let Ok(metadata) = fs::metadata(&cache_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed > self.cache_ttl {
                        return Ok(None);
                    }
                }
            }
        }

        // Read and parse cache
        let content = fs::read_to_string(&cache_file)?;
        let index: ClawHubIndex = serde_json::from_str(&content)?;

        Ok(Some(index))
    }

    /// Store the index in cache
    pub fn put(&self, index: &ClawHubIndex) -> Result<()> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&self.cache_dir)?;

        let cache_file = self.cache_dir.join("index.json");
        let content = serde_json::to_string_pretty(index)?;

        fs::write(&cache_file, content)?;

        Ok(())
    }

    /// Clear the cache
    pub fn clear(&self) -> Result<()> {
        let cache_file = self.cache_dir.join("index.json");

        if cache_file.exists() {
            fs::remove_file(&cache_file)?;
        }

        Ok(())
    }

    /// Check if cache is expired
    pub fn is_expired(&self) -> bool {
        let cache_file = self.cache_dir.join("index.json");

        if !cache_file.exists() {
            return true;
        }

        if let Ok(metadata) = fs::metadata(&cache_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    return elapsed > self.cache_ttl;
                }
            }
        }

        false
    }
}

impl Default for ClawHubCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            cache_dir: PathBuf::from(".cache/zeroclaw/clawhub"),
            cache_ttl: DEFAULT_CACHE_TTL,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_default() {
        let cache = ClawHubCache::default();
        // Should not panic and should have a valid cache dir
        // The exact path depends on the system, so we just check it's not empty
        assert!(!cache.cache_dir.as_os_str().is_empty());
    }

    #[test]
    fn test_cache_with_custom_dir() {
        let custom_dir = PathBuf::from("/tmp/test_cache");
        let cache = ClawHubCache::with_cache_dir(custom_dir.clone());

        assert_eq!(cache.cache_dir, custom_dir);
    }

    #[test]
    fn test_cache_with_custom_ttl() {
        let cache = ClawHubCache::default().with_cache_ttl(Duration::from_secs(7200));

        assert_eq!(cache.cache_ttl, Duration::from_secs(7200));
    }

    #[test]
    fn test_cache_put_and_get() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        // Put should succeed
        assert!(cache.put(&index).is_ok());

        // Get should return the index
        let retrieved = cache.get().unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), index);
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        cache.put(&index).unwrap();
        assert!(cache.get().unwrap().is_some());

        cache.clear().unwrap();
        assert!(cache.get().unwrap().is_none());
    }
}
