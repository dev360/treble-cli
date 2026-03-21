//! `treble status` — check authentication and project state
//!
//! Machine-readable with --json for agent consumption.

use crate::config::{find_project_root, GlobalConfig, ProjectConfig};
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub async fn run(json_output: bool) -> Result<()> {
    let config = GlobalConfig::load()?;

    // Check if we're in a treble project
    let project = find_project_root()
        .ok()
        .and_then(|root| ProjectConfig::load(&root).ok().map(|pc| (root, pc)));

    let project_path = project.as_ref().map(|(root, _)| root.as_path());

    // Resolve active account for this project
    let active_account = config.resolve_account(project_path).ok();

    // Validate active account's token against Figma API
    let mut token_valid = false;
    let mut api_email = None;
    let mut api_handle = None;

    if let Some(account) = active_account {
        let client = if account.auth_type == "oauth" {
            crate::figma::client::FigmaClient::new_oauth(&account.figma_token)
        } else {
            crate::figma::client::FigmaClient::new(&account.figma_token)
        };
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
        let has_token = active_account.is_some();
        let mut result = json!({
            "authenticated": has_token && token_valid,
            "hasToken": has_token,
            "tokenValid": token_valid,
        });

        if let Some(account) = active_account {
            result["activeAccount"] = json!(account.name);
        }
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

        // All accounts
        let accounts_json: Vec<_> = config
            .accounts
            .iter()
            .map(|a| {
                json!({
                    "name": a.name,
                    "authType": a.auth_type,
                    "email": a.user_email,
                    "isDefault": config.default_account.as_deref() == Some(&a.name),
                })
            })
            .collect();
        result["accounts"] = json!(accounts_json);

        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Human-readable output
    println!("{}", "treble status".bold());
    println!();

    // Show accounts
    if config.accounts.is_empty() {
        println!("  {} No accounts configured", "Auth:".yellow());
        println!("  Run: {}", "treble login --pat".cyan());
    } else if config.accounts.len() == 1 {
        let account = &config.accounts[0];
        if !token_valid {
            println!("  {} Token is invalid or expired", "Auth:".red());
            println!("  Run: {}", "treble login --pat".cyan());
        } else {
            let identity = api_handle
                .as_deref()
                .or(api_email.as_deref())
                .or(account.user_name.as_deref())
                .or(account.user_email.as_deref())
                .unwrap_or("unknown");
            println!(
                "  {} Logged in as {} ({})",
                "Auth:".green(),
                identity.white().bold(),
                account.name.cyan()
            );
        }
    } else {
        println!("  {} {} accounts", "Auth:".green(), config.accounts.len());
        for account in &config.accounts {
            let is_default = config.default_account.as_deref() == Some(&account.name);
            let is_active = active_account
                .map(|a| a.name == account.name)
                .unwrap_or(false);

            let markers = match (is_active, is_default) {
                (true, true) => " (active, default)".dimmed().to_string(),
                (true, false) => " (active)".dimmed().to_string(),
                (false, true) => " (default)".dimmed().to_string(),
                (false, false) => String::new(),
            };

            let email = account.user_email.as_deref().unwrap_or("");
            println!(
                "    {} {}{} — {} [{}]",
                if is_active { ">" } else { " " },
                account.name.cyan(),
                markers,
                email,
                account.auth_type.dimmed()
            );
        }
    }

    if let Some((root, pc)) = &project {
        println!(
            "  {} {} ({})",
            "Project:".green(),
            root.display(),
            pc.figma_file_key.dimmed()
        );

        // Show project binding
        if let Some(account) = active_account {
            let is_bound = config
                .project_bindings
                .iter()
                .any(|b| b.account == account.name);
            if is_bound {
                println!(
                    "  {} Bound to account {}",
                    "Binding:".green(),
                    account.name.cyan()
                );
            }
        }

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
