use zeroclaw::clawhub::ClawHubScraper;
use clap::Subcommand;
use std::path::PathBuf;

/// Skills management commands for ClawHub integration
#[derive(Subcommand, Debug, Clone)]
pub enum SkillsCommand {
    /// Search for skills by query
    Search {
        /// Search query string
        query: String,
    },
    /// Show trending skills
    Trending {
        /// Limit number of results (optional)
        #[arg(long)]
        limit: Option<usize>,
    },
    /// List all available skills from ClawHub
    List,
    /// Update the local skill index cache
    Update,
    /// Show detailed information about a specific skill
    Info {
        /// Skill name
        name: String,
    },
    /// Install a skill from ClawHub
    Install {
        /// Skill slug (e.g., "steipete/trello" or just "trello")
        name: String,
    },
}

/// Format skill entry for display
fn format_skill_entry(index: usize, skill: &zeroclaw::clawhub::SkillIndexEntry) -> String {
    format!(
        "{}. \x1b[1;36m{}\x1b[0m v{} ({} downloads)\n   {}",
        index,
        skill.name,
        skill.version,
        skill.downloads,
        skill.description
    )
}

/// Format detailed skill information
fn format_skill_info(skill: &zeroclaw::clawhub::SkillIndexEntry) -> String {
    format!(
        "\x1b[1;36m{}\x1b[0m
  Version: {}
  Downloads: {}
  Source: {}
  Repository: {}
  Tags: {}

  \x1b[1;33mDescription:\x1b[0m {}",
        skill.name,
        skill.version,
        skill.downloads,
        skill.source_project,
        skill.repository,
        if skill.tags.is_empty() {
            "None".to_string()
        } else {
            skill.tags.join(", ")
        },
        skill.description
    )
}

/// Handle skills commands
pub async fn handle_skills_command(
    command: SkillsCommand,
    _workspace_dir: &PathBuf,
) -> anyhow::Result<()> {
    match command {
        SkillsCommand::Search { query } => {
            println!("Searching for skills matching: \x1b[1;33m{}\x1b[0m", query);
            println!("Fetching data from clawhub.ai...");
            println!();

            let query_clone = query.clone();
            let results = tokio::task::spawn_blocking(move || {
                let scraper = ClawHubScraper::new()?;
                scraper.search_skills(&query_clone)
            })
            .await??;

            if results.is_empty() {
                println!("No skills found matching '{}'", query);
                return Ok(());
            }

            println!("Found \x1b[1;32m{}\x1b[0m skill(s):\n", results.len());
            for (index, skill) in results.iter().enumerate() {
                println!("{}\n", format_skill_entry(index + 1, skill));
            }

            Ok(())
        }
        SkillsCommand::Trending { limit } => {
            let display_limit = limit.unwrap_or(10);
            println!("Top \x1b[1;32m{}\x1b[0m trending skills on ClawHub:\n", display_limit);
            println!("Fetching data from clawhub.ai...");
            println!();

            let results = tokio::task::spawn_blocking(move || {
                let scraper = ClawHubScraper::new()?;
                let index = scraper.fetch_popular_skills(Some(display_limit))?;
                Ok::<Vec<zeroclaw::clawhub::SkillIndexEntry>, anyhow::Error>(index.skills)
            })
            .await??;

            if results.is_empty() {
                println!("No trending skills available");
                return Ok(());
            }

            for (index, skill) in results.iter().enumerate() {
                println!("{}\n", format_skill_entry(index + 1, skill));
            }

            Ok(())
        }
        SkillsCommand::List => {
            println!("All available skills on ClawHub:");
            println!("Fetching data from clawhub.ai...");
            println!();

            let index = tokio::task::spawn_blocking(|| {
                let scraper = ClawHubScraper::new()?;
                scraper.fetch_popular_skills(Some(100))
            })
            .await??;

            if index.skills.is_empty() {
                println!("No skills available");
                return Ok(());
            }

            println!("Total: \x1b[1;32m{}\x1b[0m skill(s)", index.skills.len());
            println!("Last updated: \x1b[1;36m{}\x1b[0m\n", index.last_updated);

            for (index, skill) in index.skills.iter().enumerate() {
                println!("{}\n", format_skill_entry(index + 1, skill));
            }

            Ok(())
        }
        SkillsCommand::Update => {
            println!("Fetching fresh data from ClawHub...");
            println!();

            match tokio::task::spawn_blocking(|| {
                let scraper = ClawHubScraper::new()?;
                scraper.fetch_popular_skills(Some(50))
            })
            .await?
            {
                Ok(index) => {
                    println!(
                        "\x1b[1;32m✓\x1b[0m Data updated successfully!\n",
                    );
                    println!("  Skills: {}", index.skills.len());
                    println!("  Last updated: {}", index.last_updated);
                    println!("\nNote: This is a live fetch from clawhub.ai");
                    println!("For persistent caching, consider using the local skills command:");
                    println!("  zeroclaw skills list");
                    Ok(())
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("Missing dependency") || error_msg.contains("clawhub CLI") {
                        eprintln!("\x1b[1;31m✗\x1b[0m clawhub CLI is not installed");
                        eprintln!("\nTo use this feature, install the clawhub CLI:");
                        eprintln!("  npm install -g clawhub");
                        eprintln!("\nOr visit clawhub.ai directly in your browser:");
                        eprintln!("  https://clawhub.ai/skills");
                    } else {
                        eprintln!("\x1b[1;31m✗\x1b[0m Failed to fetch data: {}", e);
                        eprintln!("\nTip: Check your internet connection");
                    }
                    Err(e.into())
                }
            }
        }
        SkillsCommand::Info { name } => {
            println!("Fetching skill info from clawhub.ai...");
            println!();

            let name_clone = name.clone();
            let skill = tokio::task::spawn_blocking(move || {
                let scraper = ClawHubScraper::new()?;
                scraper.get_skill(&name_clone)
            })
            .await??;

            if let Some(skill) = skill {
                println!("{}\n", format_skill_info(&skill));
                println!("View on ClawHub: {}", skill.repository);
                Ok(())
            } else {
                println!("Skill '\x1b[1;33m{}\x1b[0m' not found on ClawHub", name);
                println!("\nUse \x1b[1;36mzeroclaw claw-hub-skills search {}\x1b[0m to search for similar skills", name);
                println!("Or visit: https://clawhub.ai/skills");
                Ok(())
            }
        }
        SkillsCommand::Install { name } => {
            println!("Installing skill '\x1b[1;33m{}\x1b[0m' from ClawHub...", name);
            println!();

            let name_clone = name.clone();
            let workspace_dir_clone = _workspace_dir.clone();
            let result = tokio::task::spawn_blocking(move || {
                let scraper = zeroclaw::clawhub::ClawHubScraper::new()?;
                scraper.install_skill(&name_clone, &workspace_dir_clone)
            })
            .await?;

            match result {
                Ok(skill_path) => {
                    println!(
                        "\x1b[1;32m✓\x1b[0m Skill installed successfully!\n",
                    );
                    println!("  Location: {}", skill_path.display());
                    println!("\nSkill will be available after restarting ZeroClaw.");
                    println!("Or use \x1b[1;36mzeroclaw skills list\x1b[0m to see all installed skills.");
                    Ok(())
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("Missing dependency") || error_msg.contains("clawhub CLI") {
                        eprintln!("\x1b[1;31m✗\x1b[0m clawhub CLI is not installed");
                        eprintln!("\nTo use this feature, install the clawhub CLI:");
                        eprintln!("  npm install -g clawhub");
                        eprintln!("\nAlternatively, you can:");
                        println!("  1. Visit https://clawhub.ai/skills");
                        println!("  2. Find the skill's GitHub repository");
                        println!("  3. Install with: zeroclaw skills install <github-url>");
                    } else {
                        eprintln!("\x1b[1;31m✗\x1b[0m Failed to install skill: {}", e);
                        eprintln!("\nTip: Make sure the skill name is correct (e.g., 'steipete/trello' or 'trello')");
                    }
                    Err(e.into())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroclaw::clawhub::{ClawHubCache, ClawHubIndex, SkillIndexEntry};
    use std::path::PathBuf;

    /// Create a test client with mock data
    fn create_test_client() -> ClawHubClient {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = ClawHubCache::with_cache_dir(temp_dir.path().to_path_buf());

        let index = ClawHubIndex {
            skills: vec![
                SkillIndexEntry {
                    name: "test-skill".to_string(),
                    description: "A test skill for demonstration".to_string(),
                    version: "1.0.0".to_string(),
                    downloads: 1000,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/repo".to_string(),
                    tags: vec!["test".to_string(), "demo".to_string()],
                },
                SkillIndexEntry {
                    name: "automation-skill".to_string(),
                    description: "Automation helpers for workflows".to_string(),
                    version: "2.1.0".to_string(),
                    downloads: 500,
                    source_project: "zeroclaw".to_string(),
                    repository: "https://github.com/test/automation".to_string(),
                    tags: vec!["automation".to_string(), "productivity".to_string()],
                },
            ],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        cache.put(&index).unwrap();
        ClawHubClient::default().with_cache(cache)
    }

    #[test]
    fn test_handle_search() {
        let client = create_test_client();
        let result = handle_skills_command(
            SkillsCommand::Search {
                query: "test".to_string(),
            },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_search_no_results() {
        let client = create_test_client();
        let result = handle_skills_command(
            SkillsCommand::Search {
                query: "nonexistent".to_string(),
            },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_list() {
        let client = create_test_client();
        let result = handle_skills_command(SkillsCommand::List, &PathBuf::from("/tmp/workspace"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_trending() {
        let client = create_test_client();
        let result = handle_skills_command(
            SkillsCommand::Trending { limit: Some(5) },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_trending_with_default_limit() {
        let client = create_test_client();
        let result = handle_skills_command(
            SkillsCommand::Trending { limit: None },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_info_found() {
        let client = create_test_client();
        let result = handle_skills_command(
            SkillsCommand::Info {
                name: "test-skill".to_string(),
            },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_info_not_found() {
        let client = create_test_client();
        let result = handle_skills_command(
            SkillsCommand::Info {
                name: "nonexistent".to_string(),
            },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_update() {
        let client = create_test_client();
        let result = handle_skills_command(SkillsCommand::Update, &PathBuf::from("/tmp/workspace"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_skill_entry() {
        let skill = SkillIndexEntry {
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            version: "1.0.0".to_string(),
            downloads: 100,
            source_project: "zeroclaw".to_string(),
            repository: "https://github.com/test/repo".to_string(),
            tags: vec!["test".to_string()],
        };

        let formatted = format_skill_entry(1, &skill);
        assert!(formatted.contains("test-skill"));
        assert!(formatted.contains("1.0.0"));
        assert!(formatted.contains("100"));
    }

    #[test]
    fn test_format_skill_info() {
        let skill = SkillIndexEntry {
            name: "test-skill".to_string(),
            description: "A test skill for testing".to_string(),
            version: "1.0.0".to_string(),
            downloads: 100,
            source_project: "zeroclaw".to_string(),
            repository: "https://github.com/test/repo".to_string(),
            tags: vec!["test".to_string(), "demo".to_string()],
        };

        let formatted = format_skill_info(&skill);
        assert!(formatted.contains("test-skill"));
        assert!(formatted.contains("1.0.0"));
        assert!(formatted.contains("100"));
        assert!(formatted.contains("test, demo"));
        assert!(formatted.contains("A test skill for testing"));
    }
}
