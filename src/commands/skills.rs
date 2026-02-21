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
    /// List all available skills
    List,
    /// Update the local skill index cache
    Update,
    /// Show detailed information about a specific skill
    Info {
        /// Skill name
        name: String,
    },
}

/// Handle skills commands
pub fn handle_skills_command(
    command: SkillsCommand,
    _workspace_dir: &PathBuf,
) -> anyhow::Result<()> {
    // Import ClawHub types from library
    // Note: commands module is part of the library, so use crate:: prefix

    use crate::clawhub::{ClawHubClient, types::SkillIndexEntry};

    let client = ClawHubClient::new()?;

    match command {
        SkillsCommand::Search { query } => {
            println!("Searching for skills matching: {query}");

            let result: Result<Vec<SkillIndexEntry>, _> = client.search(&query);
            match result {
                Ok(results) => {
                    if results.is_empty() {
                        println!("No skills found matching '{query}'");
                    } else {
                        println!("Found {} skill(s):", results.len());
                        for skill in results {
                            println!("  • {} (v{})", skill.name, skill.version);
                            println!("    {}", skill.description);
                            if !skill.tags.is_empty() {
                                println!("    Tags: {}", skill.tags.join(", "));
                            }
                            println!("    Downloads: {}", skill.downloads);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error searching skills: {e}");
                }
            }

            Ok(())
        }
        SkillsCommand::Trending { limit } => {
            let display_limit = limit.unwrap_or(10);
            println!("Top {display_limit} trending skills:");

            let trending_result: Result<Vec<SkillIndexEntry>, _> = client.get_trending(limit);
            match trending_result {
                Ok(skills) => {
                    if skills.is_empty() {
                        println!("No trending skills available");
                    } else {
                        for (idx, skill) in skills.iter().enumerate() {
                            println!("  {}. {} (v{})", idx + 1, skill.name, skill.version);
                            println!("     {}", skill.description);
                            println!("     Downloads: {}", skill.downloads);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching trending skills: {e}");
                }
            }

            Ok(())
        }
        SkillsCommand::List => {
            println!("Available skills:");

            match client.get_index() {
                Ok(index) => {
                    if index.skills.is_empty() {
                        println!("No skills available");
                    } else {
                        println!("Total: {} skill(s)", index.skills.len());
                        println!();

                        // Group by source project
                        let mut by_source: std::collections::HashMap<String, Vec<_>> =
                            std::collections::HashMap::new();
                        for skill in &index.skills {
                            by_source
                                .entry(skill.source_project.clone())
                                .or_default()
                                .push(skill);
                        }

                        for (source, skills) in by_source.iter() {
                            println!("{source}:");
                            for skill in skills {
                                println!("  • {} (v{}) - {} downloads", skill.name, skill.version, skill.downloads);
                            }
                            println!();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching skill index: {e}");
                }
            }

            Ok(())
        }
        SkillsCommand::Update => {
            println!("Updating skill index cache...");

            match client.refresh_index() {
                Ok(index) => {
                    println!("Updated successfully!");
                    println!("Total skills: {}", index.skills.len());
                    println!("Last updated: {}", index.last_updated);
                }
                Err(e) => {
                    eprintln!("Error updating index: {e}");
                }
            }

            Ok(())
        }
        SkillsCommand::Info { name } => {
            println!("Skill information for: {name}");

            match client.get_skill(&name) {
                Ok(Some(skill)) => {
                    println!("Name: {}", skill.name);
                    println!("Version: {}", skill.version);
                    println!("Description: {}", skill.description);
                    println!("Source: {}", skill.source_project);
                    println!("Repository: {}", skill.repository);
                    println!("Downloads: {}", skill.downloads);
                    if !skill.tags.is_empty() {
                        println!("Tags: {}", skill.tags.join(", "));
                    }
                }
                Ok(None) => {
                    println!("Skill '{name}' not found");
                }
                Err(e) => {
                    eprintln!("Error fetching skill info: {e}");
                }
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_search_empty_query() {
        let result = handle_skills_command(
            SkillsCommand::Search {
                query: "test".to_string(),
            },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_list() {
        let result = handle_skills_command(SkillsCommand::List, &PathBuf::from("/tmp/workspace"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_trending() {
        let result = handle_skills_command(
            SkillsCommand::Trending { limit: Some(5) },
            &PathBuf::from("/tmp/workspace"),
        );

        assert!(result.is_ok());
    }
}
