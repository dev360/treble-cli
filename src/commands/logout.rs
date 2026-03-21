//! `treble logout` — remove Figma accounts
//!
//! - `treble logout` — interactive picker if multiple accounts, or remove sole account
//! - `treble logout <name>` — remove a specific account
//! - `treble logout --all` — remove all accounts

use crate::config::GlobalConfig;
use anyhow::Result;
use colored::Colorize;

pub fn run(account_name: Option<String>, all: bool) -> Result<()> {
    let mut config = GlobalConfig::load()?;

    if config.accounts.is_empty() {
        println!("No accounts configured.");
        return Ok(());
    }

    if all {
        let count = config.accounts.len();
        config.accounts.clear();
        config.project_bindings.clear();
        config.default_account = None;
        config.save()?;
        println!(
            "{} Removed {} account{}",
            "Done!".green().bold(),
            count,
            if count == 1 { "" } else { "s" }
        );
        return Ok(());
    }

    let name = match account_name {
        Some(n) => n,
        None => {
            if config.accounts.len() == 1 {
                config.accounts[0].name.clone()
            } else {
                // Interactive picker
                let items: Vec<String> = config
                    .accounts
                    .iter()
                    .map(|a| {
                        let default_marker = if config.default_account.as_deref() == Some(&a.name) {
                            " (default)"
                        } else {
                            ""
                        };
                        let email = a.user_email.as_deref().unwrap_or("");
                        format!("{}{} — {} [{}]", a.name, default_marker, email, a.auth_type)
                    })
                    .collect();

                let selection = dialoguer::Select::new()
                    .with_prompt("Which account to remove?")
                    .items(&items)
                    .default(0)
                    .interact()?;

                config.accounts[selection].name.clone()
            }
        }
    };

    if config.remove_account(&name) {
        config.save()?;
        println!("{} Removed account {}", "Done!".green().bold(), name.cyan());
        if let Some(ref default) = config.default_account {
            println!("  Default is now: {}", default.cyan());
        }
    } else {
        println!("{} Account '{}' not found", "Error:".red().bold(), name);
        println!("  Available: {}", {
            config
                .accounts
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        });
    }

    Ok(())
}
