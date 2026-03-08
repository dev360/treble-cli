//! Global config at ~/.treble/config.toml
//! Project config at .treble/config.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Global config (~/.treble/config.toml) ───────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub figma_token: Option<String>,
    // Web auth fields (from `treble login` device flow)
    #[serde(default)]
    pub session_token: Option<String>,
    #[serde(default)]
    pub figma_refresh_token: Option<String>,
    #[serde(default)]
    pub figma_token_expires_at: Option<String>,
    #[serde(default)]
    pub user_email: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
}

impl GlobalConfig {
    pub fn path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".treble").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;

        // Set file permissions to 600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    pub fn require_figma_token(&self) -> Result<&str> {
        self.figma_token
            .as_deref()
            .context("Figma token not configured. Run `treble login` first.")
    }
}

// ── Project config (.treble/config.toml) ────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub figma_file_key: String,
    pub flavor: String,
}

impl ProjectConfig {
    pub fn load(project_dir: &Path) -> Result<Self> {
        let path = project_dir.join(".treble").join("config.toml");
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("No .treble/config.toml found. Run `treble init` first."))?;
        toml::from_str(&content).context("Failed to parse .treble/config.toml")
    }

    pub fn save(&self, project_dir: &Path) -> Result<()> {
        let path = project_dir.join(".treble").join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

/// Find the project root by walking up from cwd looking for .treble/
pub fn find_project_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".treble").is_dir() {
            return Ok(dir.to_path_buf());
        }
        dir = dir
            .parent()
            .context("Not in a treble project. Run `treble init` first.")?;
    }
}
