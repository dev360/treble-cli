//! `treble login` — store Figma token
//!
//! Three modes:
//! 1. `treble login` — device flow via treble.build (OAuth)
//! 2. `treble login --pat` — interactive PAT entry
//! 3. `treble login --figma-token <token>` — non-interactive (for scripts/agents)

use crate::config::GlobalConfig;
use crate::figma::client::FigmaClient;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::time::Duration;

const CLIENT_ID: &str = "treble-cli";
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const MAX_POLL_ATTEMPTS: u32 = 360; // 30 min at 5s intervals

// ── Device flow response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    #[serde(default)]
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FigmaTokenResponse {
    #[serde(default)]
    figma_access_token: Option<String>,
    #[serde(default)]
    figma_refresh_token: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(default)]
    user: Option<FigmaTokenUser>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FigmaTokenUser {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

// ── Entry point ─────────────────────────────────────────────────────────

pub async fn run(pat_mode: bool, figma_token: Option<String>, server: String) -> Result<()> {
    // Non-interactive: --figma-token flag
    if let Some(token) = figma_token {
        return run_token_login(&token).await;
    }

    if pat_mode {
        return run_pat_login().await;
    }

    run_device_login(&server).await
}

// ── Non-interactive token login ─────────────────────────────────────────

async fn run_token_login(token: &str) -> Result<()> {
    let mut config = GlobalConfig::load()?;
    let token = token.trim();

    print!("Validating Figma token... ");
    let client = FigmaClient::new(token);
    match client.me().await {
        Ok(me) => {
            println!(
                "{}",
                format!("{} ({})", me.handle, me.email).green()
            );
            config.figma_token = Some(token.to_string());
            config.user_email = Some(me.email);
            config.user_name = Some(me.handle);
        }
        Err(e) => {
            println!("{}", format!("Failed: {e}").red());
            return Err(e);
        }
    }

    config.save()?;
    println!(
        "{} Saved to {}",
        "Done!".green().bold(),
        GlobalConfig::path()?.display()
    );

    Ok(())
}

// ── Device authorization flow ───────────────────────────────────────────

async fn run_device_login(server: &str) -> Result<()> {
    let http = reqwest::Client::new();
    let mut config = GlobalConfig::load()?;

    println!("{}", "Authenticating with treble.build".bold());
    println!();

    // Step 1: Request a device code
    let code_url = format!("{server}/api/auth/device/code");
    let resp = match http
        .post(&code_url)
        .json(&serde_json::json!({ "clientId": CLIENT_ID }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => {
            println!(
                "  {} Could not reach {server}",
                "Note:".yellow()
            );
            println!("  Falling back to manual Figma PAT entry.\n");
            return run_pat_login().await;
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get device code ({status}): {body}");
    }

    let device_code_resp: DeviceCodeResponse = resp
        .json()
        .await
        .context("Failed to parse device code response")?;

    let poll_interval = device_code_resp
        .interval
        .map(Duration::from_secs)
        .unwrap_or(POLL_INTERVAL);

    // Step 2: Open browser
    let verification_url = format!(
        "{server}{}?code={}",
        device_code_resp.verification_uri, device_code_resp.user_code
    );

    println!(
        "  Your code: {}",
        device_code_resp.user_code.bold().cyan()
    );
    println!();
    println!("  Opening browser to {}", verification_url.underline());
    println!(
        "  {}",
        "Approve the code in your browser to continue.".dimmed()
    );
    println!();

    if let Err(e) = open::that(&verification_url) {
        eprintln!(
            "  {} Could not open browser: {e}",
            "Warning:".yellow()
        );
        println!("  Open this URL manually: {verification_url}");
        println!();
    }

    // Step 3: Poll for token
    print!("  Waiting for approval");
    let token_url = format!("{server}/api/auth/device/token");
    let mut session_token: Option<String> = None;

    for _ in 0..MAX_POLL_ATTEMPTS {
        tokio::time::sleep(poll_interval).await;
        print!(".");

        let resp = http
            .post(&token_url)
            .json(&serde_json::json!({
                "deviceCode": device_code_resp.device_code
            }))
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(_) => continue,
        };

        let token_resp: TokenResponse = match resp.json().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        if let Some(token) = token_resp.access_token {
            session_token = Some(token);
            println!(" {}", "approved!".green());
            break;
        }

        match token_resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            Some("expired_token") => {
                println!();
                anyhow::bail!("Device code expired. Run `treble login` again.");
            }
            Some("access_denied") => {
                println!();
                anyhow::bail!("Authorization denied by user.");
            }
            Some(other) => {
                println!();
                anyhow::bail!("Unexpected error: {other}");
            }
            None => continue,
        }
    }

    let session_token = session_token.context("Timed out waiting for approval")?;

    // Step 4: Get Figma token
    print!("  Fetching Figma credentials... ");
    let figma_url = format!("{server}/api/device/figma-token");
    let resp = http
        .get(&figma_url)
        .header("Authorization", format!("Bearer {session_token}"))
        .send()
        .await
        .context("Failed to fetch Figma token")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get Figma token ({status}): {body}");
    }

    let figma_resp: FigmaTokenResponse = resp
        .json()
        .await
        .context("Failed to parse Figma token response")?;

    if let Some(err) = figma_resp.error {
        anyhow::bail!("Figma token error: {err}");
    }

    let figma_token = figma_resp
        .figma_access_token
        .context("No Figma access token in response")?;

    println!("{}", "done".green());

    // Step 5: Save to config
    config.figma_token = Some(figma_token);
    config.session_token = Some(session_token);
    config.figma_refresh_token = figma_resp.figma_refresh_token;
    config.figma_token_expires_at = figma_resp.expires_at;

    if let Some(user) = figma_resp.user {
        config.user_email = user.email.clone();
        config.user_name = user.name.clone();
    }

    config.save()?;

    let identity = config
        .user_email
        .as_deref()
        .or(config.user_name.as_deref())
        .unwrap_or("unknown");

    println!();
    println!(
        "  {} Authenticated as {}",
        "Done!".green().bold(),
        identity.white().bold()
    );
    println!(
        "  Credentials saved to {}",
        GlobalConfig::path()?.display()
    );

    Ok(())
}

// ── Manual PAT flow ─────────────────────────────────────────────────────

async fn run_pat_login() -> Result<()> {
    let mut config = GlobalConfig::load()?;

    println!("{}", "Figma Personal Access Token".bold());
    println!("Create one at: https://www.figma.com/developers/api#access-tokens");
    println!("Required scopes: file_content:read, file_metadata:read\n");

    let token: String = dialoguer::Password::new()
        .with_prompt("Figma PAT")
        .interact()?;
    let token = token.trim().to_string();

    print!("Validating... ");
    let client = FigmaClient::new(&token);
    match client.me().await {
        Ok(me) => {
            println!(
                "{}",
                format!("Logged in as {} ({})", me.handle, me.email).green()
            );
            config.figma_token = Some(token);
            config.user_email = Some(me.email);
            config.user_name = Some(me.handle);
        }
        Err(e) => {
            println!("{}", format!("Failed: {e}").red());
            return Err(e);
        }
    }

    config.save()?;
    println!(
        "\n{} Saved to {}",
        "Done!".green().bold(),
        GlobalConfig::path()?.display()
    );

    Ok(())
}
