//! Global config at ~/.treble/config.toml
//! Project config at .treble/config.toml

use crate::figma::client::FigmaClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Account + multi-account config ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub figma_token: String,
    pub auth_type: String, // "pat" or "oauth"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figma_refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figma_token_expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBinding {
    pub path: String,
    pub account: String,
}

// ── Global config (~/.treble/config.toml) ───────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_account: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<Account>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_bindings: Vec<ProjectBinding>,

    // Legacy fields (kept for migration detection, never written in v2)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    figma_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    figma_refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    figma_token_expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_name: Option<String>,
}

fn default_config_version() -> u32 {
    2
}

impl GlobalConfig {
    pub fn path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".treble").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self {
                config_version: 2,
                ..Default::default()
            });
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let mut config: GlobalConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        // Migrate legacy format (no config_version, figma_token at top level)
        if config.figma_token.is_some() && config.accounts.is_empty() {
            config.migrate_legacy();
            config.save()?;
        }

        Ok(config)
    }

    /// Migrate a legacy (v1) config to the multi-account format.
    fn migrate_legacy(&mut self) {
        if let Some(token) = self.figma_token.take() {
            let is_oauth = self
                .session_token
                .as_deref()
                .map(|s| !s.is_empty())
                .unwrap_or(false);

            let email = self.user_email.take();
            let name = self.user_name.take();
            let account_name = derive_account_slug(email.as_deref(), name.as_deref());

            let account = Account {
                name: account_name.clone(),
                figma_token: token,
                auth_type: if is_oauth { "oauth" } else { "pat" }.to_string(),
                session_token: self.session_token.take(),
                figma_refresh_token: self.figma_refresh_token.take(),
                figma_token_expires_at: self.figma_token_expires_at.take(),
                user_email: email,
                user_name: name,
            };

            self.accounts.push(account);
            self.default_account = Some(account_name);
            self.config_version = 2;
        }
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

    // ── Account resolution ──────────────────────────────────────────

    /// Resolve which account to use for a given project path.
    /// Order: project binding -> default_account -> sole account -> error.
    pub fn resolve_account(&self, project_path: Option<&Path>) -> Result<&Account> {
        // 1. Project binding
        if let Some(path) = project_path {
            let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            let canonical_str = canonical.display().to_string();
            if let Some(binding) = self
                .project_bindings
                .iter()
                .find(|b| canonical_str.starts_with(&b.path))
            {
                if let Some(account) = self.accounts.iter().find(|a| a.name == binding.account) {
                    return Ok(account);
                }
            }
        }

        // 2. Default account
        if let Some(ref default_name) = self.default_account {
            if let Some(account) = self.accounts.iter().find(|a| a.name == *default_name) {
                return Ok(account);
            }
        }

        // 3. Only account
        if self.accounts.len() == 1 {
            return Ok(&self.accounts[0]);
        }

        // 4. Error
        if self.accounts.is_empty() {
            anyhow::bail!("No Figma accounts configured. Run `treble login` first.");
        }
        anyhow::bail!(
            "Multiple accounts configured but no default or project binding set.\n\
             Run `treble init` in your project to bind an account, or set a default."
        );
    }

    /// Create a FigmaClient for the resolved account.
    pub fn figma_client(&self, project_path: Option<&Path>) -> Result<FigmaClient> {
        let account = self.resolve_account(project_path)?;
        if account.auth_type == "oauth" {
            Ok(FigmaClient::new_oauth(&account.figma_token))
        } else {
            Ok(FigmaClient::new(&account.figma_token))
        }
    }

    // ── Account management ──────────────────────────────────────────

    /// Insert or update an account. If an account with the same name exists, update it.
    /// If an account with matching email exists, update it (keeping the old name).
    pub fn upsert_account(&mut self, account: Account) {
        // Check if name matches
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.name == account.name) {
            *existing = account.clone();
        }
        // Check if email matches an existing account
        else if let Some(ref email) = account.user_email {
            if let Some(existing) = self
                .accounts
                .iter_mut()
                .find(|a| a.user_email.as_deref().map(|e| e == email).unwrap_or(false))
            {
                existing.figma_token = account.figma_token;
                existing.auth_type = account.auth_type;
                existing.session_token = account.session_token;
                existing.figma_refresh_token = account.figma_refresh_token;
                existing.figma_token_expires_at = account.figma_token_expires_at;
                existing.user_email = account.user_email;
                existing.user_name = account.user_name;
            } else {
                self.accounts.push(account.clone());
            }
        } else {
            self.accounts.push(account.clone());
        }

        // Set as default if it's the only account
        if self.accounts.len() == 1 {
            self.default_account = Some(self.accounts[0].name.clone());
        }
    }

    /// Remove an account by name. Also removes its project bindings
    /// and reassigns default if needed.
    pub fn remove_account(&mut self, name: &str) -> bool {
        let before = self.accounts.len();
        self.accounts.retain(|a| a.name != name);
        if self.accounts.len() == before {
            return false;
        }

        // Clean up bindings
        self.project_bindings.retain(|b| b.account != name);

        // Reassign default
        if self.default_account.as_deref() == Some(name) {
            self.default_account = self.accounts.first().map(|a| a.name.clone());
        }

        true
    }

    /// Bind a project path to an account.
    pub fn bind_project(&mut self, path: &Path, account_name: &str) -> Result<()> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let canonical_str = canonical.display().to_string();

        // Verify account exists
        if !self.accounts.iter().any(|a| a.name == account_name) {
            anyhow::bail!("Account '{}' not found", account_name);
        }

        // Update or insert binding
        if let Some(binding) = self
            .project_bindings
            .iter_mut()
            .find(|b| b.path == canonical_str)
        {
            binding.account = account_name.to_string();
        } else {
            self.project_bindings.push(ProjectBinding {
                path: canonical_str,
                account: account_name.to_string(),
            });
        }

        Ok(())
    }
}

// ── Slug derivation ─────────────────────────────────────────────────

/// Derive an account slug from email or name.
/// `christian@atomic.health` -> `christian-atomic`
/// `Christian Toivola` -> `christian-toivola`
/// Fallback: `account-1`
pub fn derive_account_slug(email: Option<&str>, name: Option<&str>) -> String {
    if let Some(email) = email {
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() == 2 {
            let user = parts[0];
            let domain = parts[1].split('.').next().unwrap_or("unknown");
            return slugify_name(&format!("{user}-{domain}"));
        }
    }
    if let Some(name) = name {
        return slugify_name(name);
    }
    "account-1".to_string()
}

fn slugify_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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
        let content = std::fs::read_to_string(&path).with_context(|| {
            "No .treble/config.toml found. Run `treble init` first.".to_string()
        })?;
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

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_slug_from_email() {
        assert_eq!(
            derive_account_slug(Some("christian@atomic.health"), None),
            "christian-atomic"
        );
    }

    #[test]
    fn test_derive_slug_from_gmail() {
        assert_eq!(
            derive_account_slug(Some("christian.t@gmail.com"), None),
            "christian-t-gmail"
        );
    }

    #[test]
    fn test_derive_slug_from_name() {
        assert_eq!(
            derive_account_slug(None, Some("Christian Toivola")),
            "christian-toivola"
        );
    }

    #[test]
    fn test_derive_slug_fallback() {
        assert_eq!(derive_account_slug(None, None), "account-1");
    }

    #[test]
    fn test_upsert_account_insert() {
        let mut config = GlobalConfig {
            config_version: 2,
            ..Default::default()
        };
        config.upsert_account(Account {
            name: "work".to_string(),
            figma_token: "tok1".to_string(),
            auth_type: "pat".to_string(),
            session_token: None,
            figma_refresh_token: None,
            figma_token_expires_at: None,
            user_email: Some("a@b.com".to_string()),
            user_name: None,
        });
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.default_account, Some("work".to_string()));
    }

    #[test]
    fn test_upsert_account_update_by_name() {
        let mut config = GlobalConfig {
            config_version: 2,
            accounts: vec![Account {
                name: "work".to_string(),
                figma_token: "old".to_string(),
                auth_type: "pat".to_string(),
                session_token: None,
                figma_refresh_token: None,
                figma_token_expires_at: None,
                user_email: Some("a@b.com".to_string()),
                user_name: None,
            }],
            default_account: Some("work".to_string()),
            ..Default::default()
        };
        config.upsert_account(Account {
            name: "work".to_string(),
            figma_token: "new".to_string(),
            auth_type: "pat".to_string(),
            session_token: None,
            figma_refresh_token: None,
            figma_token_expires_at: None,
            user_email: Some("a@b.com".to_string()),
            user_name: None,
        });
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.accounts[0].figma_token, "new");
    }

    #[test]
    fn test_upsert_account_update_by_email() {
        let mut config = GlobalConfig {
            config_version: 2,
            accounts: vec![Account {
                name: "work".to_string(),
                figma_token: "old".to_string(),
                auth_type: "pat".to_string(),
                session_token: None,
                figma_refresh_token: None,
                figma_token_expires_at: None,
                user_email: Some("a@b.com".to_string()),
                user_name: None,
            }],
            default_account: Some("work".to_string()),
            ..Default::default()
        };
        config.upsert_account(Account {
            name: "different-name".to_string(),
            figma_token: "new".to_string(),
            auth_type: "pat".to_string(),
            session_token: None,
            figma_refresh_token: None,
            figma_token_expires_at: None,
            user_email: Some("a@b.com".to_string()),
            user_name: None,
        });
        // Should update existing, not insert
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.accounts[0].name, "work"); // keeps old name
        assert_eq!(config.accounts[0].figma_token, "new");
    }

    #[test]
    fn test_remove_account() {
        let mut config = GlobalConfig {
            config_version: 2,
            accounts: vec![
                Account {
                    name: "work".to_string(),
                    figma_token: "t1".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
                Account {
                    name: "personal".to_string(),
                    figma_token: "t2".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
            ],
            default_account: Some("work".to_string()),
            project_bindings: vec![ProjectBinding {
                path: "/some/path".to_string(),
                account: "work".to_string(),
            }],
            ..Default::default()
        };

        assert!(config.remove_account("work"));
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.accounts[0].name, "personal");
        assert_eq!(config.default_account, Some("personal".to_string()));
        assert!(config.project_bindings.is_empty());
    }

    #[test]
    fn test_remove_account_not_found() {
        let mut config = GlobalConfig {
            config_version: 2,
            ..Default::default()
        };
        assert!(!config.remove_account("nonexistent"));
    }

    #[test]
    fn test_resolve_single_account() {
        let config = GlobalConfig {
            config_version: 2,
            accounts: vec![Account {
                name: "work".to_string(),
                figma_token: "tok".to_string(),
                auth_type: "pat".to_string(),
                session_token: None,
                figma_refresh_token: None,
                figma_token_expires_at: None,
                user_email: None,
                user_name: None,
            }],
            ..Default::default()
        };
        let account = config.resolve_account(None).unwrap();
        assert_eq!(account.name, "work");
    }

    #[test]
    fn test_resolve_no_accounts_errors() {
        let config = GlobalConfig {
            config_version: 2,
            ..Default::default()
        };
        assert!(config.resolve_account(None).is_err());
    }

    #[test]
    fn test_resolve_multiple_no_default_errors() {
        let config = GlobalConfig {
            config_version: 2,
            accounts: vec![
                Account {
                    name: "a".to_string(),
                    figma_token: "t1".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
                Account {
                    name: "b".to_string(),
                    figma_token: "t2".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
            ],
            ..Default::default()
        };
        assert!(config.resolve_account(None).is_err());
    }

    #[test]
    fn test_resolve_default_account() {
        let config = GlobalConfig {
            config_version: 2,
            default_account: Some("b".to_string()),
            accounts: vec![
                Account {
                    name: "a".to_string(),
                    figma_token: "t1".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
                Account {
                    name: "b".to_string(),
                    figma_token: "t2".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
            ],
            ..Default::default()
        };
        let account = config.resolve_account(None).unwrap();
        assert_eq!(account.name, "b");
    }

    #[test]
    fn test_legacy_migration() {
        let mut config = GlobalConfig {
            config_version: 0,
            figma_token: Some("figd_test".to_string()),
            user_email: Some("christian@atomic.health".to_string()),
            user_name: Some("Christian".to_string()),
            session_token: None,
            ..Default::default()
        };
        config.migrate_legacy();
        assert_eq!(config.config_version, 2);
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.accounts[0].name, "christian-atomic");
        assert_eq!(config.accounts[0].auth_type, "pat");
        assert_eq!(config.accounts[0].figma_token, "figd_test");
        assert_eq!(config.default_account, Some("christian-atomic".to_string()));
        // Legacy fields should be cleared
        assert!(config.figma_token.is_none());
    }

    #[test]
    fn test_legacy_migration_oauth() {
        let mut config = GlobalConfig {
            config_version: 0,
            figma_token: Some("token".to_string()),
            session_token: Some("session".to_string()),
            figma_refresh_token: Some("refresh".to_string()),
            user_email: Some("user@company.co".to_string()),
            ..Default::default()
        };
        config.migrate_legacy();
        assert_eq!(config.accounts[0].auth_type, "oauth");
        assert_eq!(
            config.accounts[0].session_token,
            Some("session".to_string())
        );
    }

    #[test]
    fn test_resolve_with_project_binding() {
        let dir = std::env::temp_dir().join("treble-test-binding");
        let _ = std::fs::create_dir_all(&dir);
        let config = GlobalConfig {
            config_version: 2,
            accounts: vec![
                Account {
                    name: "work".to_string(),
                    figma_token: "t1".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
                Account {
                    name: "personal".to_string(),
                    figma_token: "t2".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
            ],
            default_account: Some("work".to_string()),
            project_bindings: vec![ProjectBinding {
                path: std::fs::canonicalize(&dir).unwrap().display().to_string(),
                account: "personal".to_string(),
            }],
            ..Default::default()
        };
        // Project binding overrides default
        let account = config.resolve_account(Some(&dir)).unwrap();
        assert_eq!(account.name, "personal");
        // Without path, falls back to default
        let account = config.resolve_account(None).unwrap();
        assert_eq!(account.name, "work");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_bind_project_insert_and_update() {
        let dir = std::env::temp_dir().join("treble-test-bind");
        let _ = std::fs::create_dir_all(&dir);
        let mut config = GlobalConfig {
            config_version: 2,
            accounts: vec![
                Account {
                    name: "a".to_string(),
                    figma_token: "t1".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
                Account {
                    name: "b".to_string(),
                    figma_token: "t2".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
            ],
            ..Default::default()
        };
        // Insert
        config.bind_project(&dir, "a").unwrap();
        assert_eq!(config.project_bindings.len(), 1);
        assert_eq!(config.project_bindings[0].account, "a");
        // Update same path to different account
        config.bind_project(&dir, "b").unwrap();
        assert_eq!(config.project_bindings.len(), 1);
        assert_eq!(config.project_bindings[0].account, "b");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_bind_project_nonexistent_account_errors() {
        let dir = std::env::temp_dir().join("treble-test-bind-err");
        let _ = std::fs::create_dir_all(&dir);
        let mut config = GlobalConfig {
            config_version: 2,
            ..Default::default()
        };
        assert!(config.bind_project(&dir, "nope").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_upsert_second_account_no_default_change() {
        let mut config = GlobalConfig {
            config_version: 2,
            accounts: vec![Account {
                name: "first".to_string(),
                figma_token: "t1".to_string(),
                auth_type: "pat".to_string(),
                session_token: None,
                figma_refresh_token: None,
                figma_token_expires_at: None,
                user_email: None,
                user_name: None,
            }],
            default_account: Some("first".to_string()),
            ..Default::default()
        };
        config.upsert_account(Account {
            name: "second".to_string(),
            figma_token: "t2".to_string(),
            auth_type: "pat".to_string(),
            session_token: None,
            figma_refresh_token: None,
            figma_token_expires_at: None,
            user_email: Some("x@y.com".to_string()),
            user_name: None,
        });
        assert_eq!(config.accounts.len(), 2);
        // Default should NOT change to the second account
        assert_eq!(config.default_account, Some("first".to_string()));
    }

    #[test]
    fn test_remove_non_default_keeps_default() {
        let mut config = GlobalConfig {
            config_version: 2,
            accounts: vec![
                Account {
                    name: "keep".to_string(),
                    figma_token: "t1".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
                Account {
                    name: "remove-me".to_string(),
                    figma_token: "t2".to_string(),
                    auth_type: "pat".to_string(),
                    session_token: None,
                    figma_refresh_token: None,
                    figma_token_expires_at: None,
                    user_email: None,
                    user_name: None,
                },
            ],
            default_account: Some("keep".to_string()),
            ..Default::default()
        };
        assert!(config.remove_account("remove-me"));
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.default_account, Some("keep".to_string()));
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify_name("Hello World!"), "hello-world");
        assert_eq!(slugify_name("  spaces  everywhere  "), "spaces-everywhere");
        assert_eq!(slugify_name("already-slug"), "already-slug");
        assert_eq!(slugify_name("UPPER_CASE"), "upper-case");
    }
}
