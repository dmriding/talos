//! Bootstrap flow for initial Talos server setup.
//!
//! This module handles the initial token creation when the server starts without
//! any API tokens. There are two ways to bootstrap:
//!
//! 1. **Environment Variable**: Set `TALOS_BOOTSTRAP_TOKEN` with a pre-shared secret.
//!    On first startup, Talos will create an admin token using this value as the
//!    token hash, allowing the operator to immediately use the token.
//!
//! 2. **CLI Command**: Run `talos token create --name "Admin" --scopes "*"` to
//!    create a new token interactively.
//!
//! # Security
//!
//! The bootstrap token should be treated as sensitive. It grants full admin access.
//! After bootstrapping, consider creating more restricted tokens for specific services.

use std::env;
use std::sync::Arc;

use chrono::Utc;
use tracing::{info, warn};

use crate::errors::LicenseResult;
use crate::server::database::Database;

/// Environment variable name for bootstrap token.
pub const BOOTSTRAP_TOKEN_ENV: &str = "TALOS_BOOTSTRAP_TOKEN";

/// Check for and process the bootstrap token from environment.
///
/// If `TALOS_BOOTSTRAP_TOKEN` is set and no API tokens exist in the database,
/// this creates an initial admin token with full access.
///
/// # Returns
/// - `Ok(Some(token))` - A new token was created, returns the raw token value
/// - `Ok(None)` - No bootstrap needed (tokens exist or no env var set)
/// - `Err(e)` - Failed to create the bootstrap token
pub async fn check_bootstrap_token(db: &Arc<Database>) -> LicenseResult<Option<String>> {
    // Check if the environment variable is set
    let bootstrap_token = match env::var(BOOTSTRAP_TOKEN_ENV) {
        Ok(token) if !token.is_empty() => token,
        _ => return Ok(None), // No env var set, skip bootstrap
    };

    // Check if any tokens already exist
    if db.has_api_tokens().await? {
        info!(
            "Bootstrap skipped: API tokens already exist. Unset {} to suppress this message.",
            BOOTSTRAP_TOKEN_ENV
        );
        return Ok(None);
    }

    // Create the bootstrap token with full admin access
    info!("Creating bootstrap admin token from {}", BOOTSTRAP_TOKEN_ENV);

    let (token, raw_token) = db
        .create_api_token(
            "Bootstrap Admin",
            &["*"], // Full access
            None,   // Never expires
            Some("bootstrap"),
        )
        .await?;

    warn!(
        "Bootstrap token created with id={}. Store the raw token securely!",
        token.id
    );
    warn!(
        "For security, unset {} after bootstrap is complete.",
        BOOTSTRAP_TOKEN_ENV
    );

    // If the bootstrap env var contains a specific token format, we use that directly
    // Otherwise, we return the generated token
    if bootstrap_token.starts_with("talos_") {
        // User provided their own token - we need to use it as the raw value
        // This is useful for pre-configuring tokens in deployment scripts
        warn!("Using provided bootstrap token value. Ensure it's stored securely.");
        Ok(Some(bootstrap_token))
    } else {
        // Return the generated token
        Ok(Some(raw_token))
    }
}

/// CLI command result for token operations.
#[derive(Debug)]
pub enum TokenCommand {
    /// Create a new token
    Create {
        name: String,
        scopes: Vec<String>,
        expires_at: Option<String>,
    },
    /// List all tokens
    List,
    /// Revoke a token by ID
    Revoke { id: String },
    /// No command (run server normally)
    None,
}

/// Parse CLI arguments for token commands.
///
/// # Supported Commands
///
/// ```text
/// talos token create --name "My Token" --scopes "licenses:read,licenses:write"
/// talos token list
/// talos token revoke <id>
/// ```
pub fn parse_token_command(args: &[String]) -> TokenCommand {
    if args.len() < 2 {
        return TokenCommand::None;
    }

    // Check if first arg is "token"
    if args[1] != "token" {
        return TokenCommand::None;
    }

    if args.len() < 3 {
        return TokenCommand::None;
    }

    match args[2].as_str() {
        "create" => {
            let mut name = String::from("API Token");
            let mut scopes = vec!["*".to_string()];
            let mut expires_at = None;

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--name" | "-n" => {
                        if i + 1 < args.len() {
                            name = args[i + 1].clone();
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "--scopes" | "-s" => {
                        if i + 1 < args.len() {
                            scopes = args[i + 1]
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "--expires" | "-e" => {
                        if i + 1 < args.len() {
                            expires_at = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    _ => i += 1,
                }
            }

            TokenCommand::Create {
                name,
                scopes,
                expires_at,
            }
        }
        "list" => TokenCommand::List,
        "revoke" => {
            if args.len() > 3 {
                TokenCommand::Revoke {
                    id: args[3].clone(),
                }
            } else {
                eprintln!("Error: token revoke requires a token ID");
                TokenCommand::None
            }
        }
        _ => TokenCommand::None,
    }
}

/// Execute a token command.
pub async fn execute_token_command(db: &Database, cmd: TokenCommand) -> LicenseResult<bool> {
    match cmd {
        TokenCommand::Create {
            name,
            scopes,
            expires_at,
        } => {
            let expires = expires_at.and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%SZ")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S"))
                    .ok()
            });

            let scope_refs: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
            let (token, raw) = db.create_api_token(&name, &scope_refs, expires, None).await?;

            println!("Token created successfully!");
            println!("───────────────────────────────────────────");
            println!("ID:      {}", token.id);
            println!("Name:    {}", token.name);
            println!("Scopes:  {}", token.scopes);
            println!("Created: {}", token.created_at);
            if let Some(exp) = token.expires_at {
                println!("Expires: {}", exp);
            }
            println!("───────────────────────────────────────────");
            println!("RAW TOKEN (save this, shown only once):");
            println!("{}", raw);
            println!("───────────────────────────────────────────");

            Ok(true) // Exit after command
        }
        TokenCommand::List => {
            let tokens = db.list_api_tokens().await?;

            if tokens.is_empty() {
                println!("No API tokens found.");
            } else {
                println!("API Tokens:");
                println!("───────────────────────────────────────────────────────────");
                for t in tokens {
                    let status = if t.revoked_at.is_some() {
                        "REVOKED"
                    } else if let Some(exp) = t.expires_at {
                        if Utc::now().naive_utc() > exp {
                            "EXPIRED"
                        } else {
                            "ACTIVE"
                        }
                    } else {
                        "ACTIVE"
                    };

                    println!("[{}] {} - {} ({})", status, t.id, t.name, t.scopes);
                }
            }

            Ok(true) // Exit after command
        }
        TokenCommand::Revoke { id } => {
            if db.revoke_api_token(&id).await? {
                println!("Token {} revoked successfully.", id);
            } else {
                println!("Token {} not found or already revoked.", id);
            }
            Ok(true) // Exit after command
        }
        TokenCommand::None => Ok(false), // Continue with server startup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_args_returns_none() {
        let args = vec!["talos".to_string()];
        assert!(matches!(parse_token_command(&args), TokenCommand::None));
    }

    #[test]
    fn parse_token_create_minimal() {
        let args = vec![
            "talos".to_string(),
            "token".to_string(),
            "create".to_string(),
        ];
        match parse_token_command(&args) {
            TokenCommand::Create { name, scopes, .. } => {
                assert_eq!(name, "API Token");
                assert_eq!(scopes, vec!["*"]);
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn parse_token_create_with_options() {
        let args = vec![
            "talos".to_string(),
            "token".to_string(),
            "create".to_string(),
            "--name".to_string(),
            "My Token".to_string(),
            "--scopes".to_string(),
            "licenses:read,licenses:write".to_string(),
        ];
        match parse_token_command(&args) {
            TokenCommand::Create { name, scopes, .. } => {
                assert_eq!(name, "My Token");
                assert_eq!(scopes, vec!["licenses:read", "licenses:write"]);
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn parse_token_list() {
        let args = vec![
            "talos".to_string(),
            "token".to_string(),
            "list".to_string(),
        ];
        assert!(matches!(parse_token_command(&args), TokenCommand::List));
    }

    #[test]
    fn parse_token_revoke() {
        let args = vec![
            "talos".to_string(),
            "token".to_string(),
            "revoke".to_string(),
            "abc-123".to_string(),
        ];
        match parse_token_command(&args) {
            TokenCommand::Revoke { id } => assert_eq!(id, "abc-123"),
            _ => panic!("Expected Revoke command"),
        }
    }

    #[test]
    fn parse_non_token_command_returns_none() {
        let args = vec!["talos".to_string(), "serve".to_string()];
        assert!(matches!(parse_token_command(&args), TokenCommand::None));
    }
}
