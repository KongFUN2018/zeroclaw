use anyhow::Result;
use std::path::Path;
use tracing::info;

// These types are from the library's clawhub module
// When compiling as the binary, we access them via zeroclaw::
use zeroclaw::clawhub::{SecurityScanner, SkillKeyPair, verify_skill_signature, SkillSIF};

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SecurityCommands {
    /// Scan a skill for security issues
    Scan {
        /// Path to skill directory or SIF file
        path: std::path::PathBuf,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Generate a key pair for signing skills
    GenerateKey {
        /// Output path for private key (optional, prints to stdout if not specified)
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Verify skill signature
    Verify {
        /// Path to skill SIF file
        path: std::path::PathBuf,

        /// Author public key (hex)
        #[arg(long)]
        author_key: Option<String>,

        /// Marketplace public key (hex)
        #[arg(long)]
        market_key: Option<String>,
    },
}

pub async fn handle_security_command(command: SecurityCommands) -> Result<()> {
    match command {
        SecurityCommands::Scan { path, format } => {
            scan_skill(&path, &format)?;
        }
        SecurityCommands::GenerateKey { output } => {
            generate_keypair(&output)?;
        }
        SecurityCommands::Verify { path, author_key, market_key } => {
            verify_signature(&path, author_key, market_key)?;
        }
    }
    Ok(())
}

fn scan_skill(path: &Path, format: &str) -> Result<()> {
    info!("Scanning skill at: {}", path.display());

    // Read SIF file
    let sif_json = std::fs::read_to_string(path)?;
    let sif: serde_json::Value = serde_json::from_str(&sif_json)?;

    // Convert to SkillSIF
    let skill_sif: SkillSIF = serde_json::from_value(sif)?;

    // Run security scan
    let scanner = SecurityScanner::new();
    let report = scanner.scan(&skill_sif)
        .map_err(|e| anyhow::anyhow!("Security scan error: {}", e))?;

    // Output results
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        _ => {
            if report.findings.is_empty() {
                println!("✓ No security issues found");
            } else {
                println!("Found {} security issue(s):", report.findings.len());
                for finding in &report.findings {
                    println!("\n  [{:?}] {}: {}",
                        finding.severity, finding.rule, finding.message);
                    println!("    Location: {}", finding.location);
                    println!("    Suggestion: {}", finding.suggestion);
                }
            }
            println!("\nScan duration: {}ms", report.scan_duration_ms);

            if !report.is_safe() {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn generate_keypair(output: &Option<std::path::PathBuf>) -> Result<()> {
    info!("Generating new Ed25519 key pair");
    let keypair = SkillKeyPair::generate();
    let public_key = keypair.public_key_hex();

    if let Some(path) = output {
        // Save secret key bytes
        let secret_bytes = keypair.to_secret_key_bytes();
        std::fs::write(path, &secret_bytes)?;
        println!("Generated key pair:");
        println!("  Private key: {}", path.display());
        println!("  Public key (hex): {}", public_key);
    } else {
        println!("Generated key pair:");
        println!("  Public key (hex): {}", public_key);
        println!("\nNote: Private key not saved. Use --output to save.");
    }

    Ok(())
}

fn verify_signature(path: &Path, author_key: Option<String>, market_key: Option<String>) -> Result<()> {
    info!("Verifying signature for: {}", path.display());

    let sif_json = std::fs::read_to_string(path)?;
    let sif: SkillSIF = serde_json::from_str(&sif_json)?;

    let signature = match &sif.signature {
        Some(s) => s,
        None => {
            println!("✗ No signature found in SIF");
            return Ok(());
        }
    };

    let mut all_valid = true;

    if let Some(author_sig) = &signature.author_signature {
        if let Some(ref key) = author_key {
            match verify_skill_signature(sif_json.as_bytes(), author_sig, key) {
                Ok(true) => println!("✓ Author signature: VALID"),
                Ok(false) | Err(_) => {
                    println!("✗ Author signature: INVALID");
                    all_valid = false;
                }
            }
        } else {
            println!("? Author signature present but no --author-key provided");
        }
    }

    if let Some(market_sig) = &signature.market_signature {
        if let Some(ref key) = market_key {
            match verify_skill_signature(sif_json.as_bytes(), market_sig, key) {
                Ok(true) => println!("✓ Market signature: VALID"),
                Ok(false) | Err(_) => {
                    println!("✗ Market signature: INVALID");
                    all_valid = false;
                }
            }
        } else {
            println!("? Market signature present but no --market-key provided");
        }
    }

    if all_valid {
        println!("\n✓ All verified signatures are valid");
    }

    Ok(())
}
