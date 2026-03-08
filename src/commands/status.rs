//! `treble status` — check authentication and project state
//!
//! Machine-readable with --json for agent consumption.

use crate::config::{find_project_root, GlobalConfig, ProjectConfig};
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub async fn run(json_output: bool) -> Result<()> {
    let config = GlobalConfig::load()?;

    let has_token = config.figma_token.is_some();
    let stored_email = config.user_email.clone();
    let stored_name = config.user_name.clone();

    // Check if we're in a treble project
    let project = find_project_root()
        .ok()
        .and_then(|root| ProjectConfig::load(&root).ok().map(|pc| (root, pc)));

    // Validate token against Figma API if we have one
    let mut token_valid = false;
    let mut api_email = None;
    let mut api_handle = None;

    if config.figma_token.is_some() {
        let client = config.figma_client()?;
        match client.me().await {
            Ok(me) => {
                token_valid = true;
                api_email = Some(me.email);
                api_handle = Some(me.handle);
            }
            Err(_) => {
                token_valid = false;
            }
        }
    }

    if json_output {
        let mut result = json!({
            "authenticated": has_token && token_valid,
            "hasToken": has_token,
            "tokenValid": token_valid,
        });

        if let Some(email) = &api_email {
            result["email"] = json!(email);
        }
        if let Some(handle) = &api_handle {
            result["handle"] = json!(handle);
        }
        if let Some((root, pc)) = &project {
            result["project"] = json!({
                "root": root.display().to_string(),
                "figmaFileKey": pc.figma_file_key,
            });
        }

        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Human-readable output
    println!("{}", "treble status".bold());
    println!();

    if !has_token {
        println!("  {} Not authenticated", "Auth:".yellow());
        println!("  Run: {}", "treble login --pat".cyan());
    } else if !token_valid {
        println!("  {} Token is invalid or expired", "Auth:".red());
        println!("  Run: {}", "treble login --pat".cyan());
    } else {
        let identity = api_handle
            .as_deref()
            .or(api_email.as_deref())
            .or(stored_name.as_deref())
            .or(stored_email.as_deref())
            .unwrap_or("unknown");
        println!(
            "  {} Logged in as {}",
            "Auth:".green(),
            identity.white().bold()
        );
    }

    if let Some((root, pc)) = &project {
        println!(
            "  {} {} ({})",
            "Project:".green(),
            root.display(),
            pc.figma_file_key.dimmed()
        );

        // Check if any frames are synced
        let figma_dir = root.join(".treble").join("figma");
        let manifest_path = figma_dir.join("manifest.json");
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)?;
            let manifest: crate::figma::types::FigmaManifest = serde_json::from_str(&content)?;
            println!(
                "  {} {} frames synced",
                "Synced:".green(),
                manifest.frames.len()
            );
        } else {
            println!("  {} No frames synced yet", "Synced:".yellow());
            println!("  Run: {}", "treble sync".cyan());
        }
    } else {
        println!("  {} Not in a treble project", "Project:".dimmed());
    }

    Ok(())
}
