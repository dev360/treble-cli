//! `treble init` — scaffold .treble/ in current project directory

use crate::config::{GlobalConfig, ProjectConfig};
use crate::figma::client::FigmaClient;
use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::Input;

pub async fn run(figma_arg: Option<String>, flavor: String) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let treble_dir = cwd.join(".treble");

    if treble_dir.exists() {
        println!(
            "{} .treble/ already exists in {}",
            "Warning:".yellow(),
            cwd.display()
        );
    }

    // ── Get Figma file key ──────────────────────────────────────────────
    let file_key = match figma_arg {
        Some(input) => extract_file_key(&input),
        None => {
            let input: String = Input::new()
                .with_prompt("Figma file URL or key")
                .interact_text()?;
            extract_file_key(&input)
        }
    };

    // Validate the file key
    let config = GlobalConfig::load()?;
    let token = match config.figma_token.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => {
            println!("\n  {} No Figma token found.\n", "Error:".red().bold());
            println!("  Run one of these first:\n");
            println!("    {}  Sign in via treble.build", "treble login".bold());
            println!(
                "    {}  Enter a Personal Access Token",
                "treble login --pat".bold()
            );
            println!(
                "    {}  Non-interactive\n",
                "treble login --figma-token <TOKEN>".bold()
            );
            println!(
                "  Create a PAT at: {}\n",
                "https://www.figma.com/developers/api#access-tokens".underline()
            );
            std::process::exit(1);
        }
    };
    let client = FigmaClient::new(token);

    print!("Validating Figma file... ");
    match client.get_file(&file_key).await {
        Ok(file) => {
            println!("{}", format!("\"{}\"", file.name).green());

            // Show pages
            for page in &file.document.children {
                let frame_count = page.children.len();
                println!("  {} {} ({} frames)", "→".dimmed(), page.name, frame_count);
            }
        }
        Err(e) => {
            println!("{}", format!("Failed: {e}").red());
            return Err(e);
        }
    }

    // ── Create .treble/ structure ───────────────────────────────────────
    std::fs::create_dir_all(treble_dir.join("figma")).context("Failed to create .treble/figma/")?;

    // Write project config
    let project_config = ProjectConfig {
        figma_file_key: file_key.clone(),
        flavor: flavor.clone(),
    };
    project_config.save(&cwd)?;

    println!(
        "\n{} Initialized .treble/ in {}",
        "Done!".green().bold(),
        cwd.display()
    );
    println!("  File key: {}", file_key.dimmed());
    println!("  Flavor:   {}", flavor.dimmed());
    println!(
        "\nNext: run {} to pull Figma data to disk",
        "treble sync".bold()
    );

    Ok(())
}

/// Extract a Figma file key from a URL or raw key string.
/// Handles:
///   - "abc123DEFghiJKL" (raw key)
///   - "https://www.figma.com/design/abc123DEFghiJKL/My-Design"
///   - "https://www.figma.com/file/abc123DEFghiJKL"
fn extract_file_key(input: &str) -> String {
    let input = input.trim();

    // If it contains figma.com, parse the URL
    if input.contains("figma.com") {
        // Split by / and find the segment after "design" or "file"
        let parts: Vec<&str> = input.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if (*part == "design" || *part == "file") && i + 1 < parts.len() {
                // The next segment is the key (may have query params)
                return parts[i + 1]
                    .split('?')
                    .next()
                    .unwrap_or(parts[i + 1])
                    .to_string();
            }
        }
    }

    // Otherwise treat as raw key
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_key_raw() {
        assert_eq!(extract_file_key("abc123DEFghiJKL"), "abc123DEFghiJKL");
    }

    #[test]
    fn test_extract_file_key_design_url() {
        assert_eq!(
            extract_file_key("https://www.figma.com/design/abc123DEFghiJKL/My-Design"),
            "abc123DEFghiJKL"
        );
    }

    #[test]
    fn test_extract_file_key_file_url() {
        assert_eq!(
            extract_file_key("https://www.figma.com/file/abc123DEFghiJKL"),
            "abc123DEFghiJKL"
        );
    }
}
