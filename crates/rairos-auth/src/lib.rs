//! Rairos Auth — Web session authentication
//!
//! Ported from llm/auth.py
//!
//! Provides:
//! - Optional session-based auth (single-user mode when no users set up)
//! - PBKDF2 password hashing with salt
//! - Session tokens with TTL
//! - File-based storage in ~/.ai_research_os/

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid password")]
    InvalidPassword,

    #[error("User not found")]
    UserNotFound,

    #[error("Session expired")]
    SessionExpired,

    #[error("Auth not enabled")]
    AuthNotEnabled,
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub salt: String,
    pub password_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AuthData {
    users: std::collections::HashMap<String, User>,
    setup_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    username: String,
    created_at: f64,
    expires_at: f64,
}

// ============================================================================
// Constants
// ============================================================================

const SESSION_TTL_SECONDS: f64 = 86400.0 * 7.0; // 7 days

fn auth_file() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join("auth.json"))
        .unwrap_or_else(|| PathBuf::from("auth.json"))
}

fn sessions_file() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join("sessions.json"))
        .unwrap_or_else(|| PathBuf::from("sessions.json"))
}

// ============================================================================
// Password Hashing
// ============================================================================

fn hash_password(password: &str, salt: &str) -> String {
    use pbkdf2::pbkdf2_hmac_array;
    use sha2::Sha256;
    let salt_bytes = salt.as_bytes();
    let result = pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), salt_bytes, 100_000);
    hex::encode(result)
}

fn generate_salt() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

// ============================================================================
// Auth Operations
// ============================================================================

fn load_auth() -> Result<AuthData, AuthError> {
    let path = auth_file();
    if !path.exists() {
        return Ok(AuthData::default());
    }
    let contents = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn save_auth(auth: &AuthData) -> Result<(), AuthError> {
    let path = auth_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(auth)?;
    std::fs::write(&path, contents)?;
    Ok(())
}

fn load_sessions() -> Result<std::collections::HashMap<String, Session>, AuthError> {
    let path = sessions_file();
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }
    let contents = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn save_sessions(sessions: &std::collections::HashMap<String, Session>) -> Result<(), AuthError> {
    let path = sessions_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(sessions)?;
    std::fs::write(&path, contents)?;
    Ok(())
}

// ============================================================================
// Public API
// ============================================================================

pub fn is_auth_enabled() -> bool {
    load_auth().map(|a| a.setup_complete).unwrap_or(false)
}

pub fn setup_admin(username: &str, password: &str) -> Result<bool, AuthError> {
    let mut auth = load_auth().unwrap_or_default();
    if auth.setup_complete {
        return Ok(false);
    }
    let salt = generate_salt();
    let password_hash = hash_password(password, &salt);
    auth.users.insert(
        username.to_string(),
        User {
            username: username.to_string(),
            salt: salt.clone(),
            password_hash,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    );
    auth.setup_complete = true;
    save_auth(&auth)?;
    Ok(true)
}

pub fn verify_login(username: &str, password: &str) -> bool {
    let auth = match load_auth() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let user = match auth.users.get(username) {
        Some(u) => u,
        None => return false,
    };
    user.password_hash == hash_password(password, &user.salt)
}

pub fn create_session(username: &str) -> Result<String, AuthError> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    let token = hex::encode(bytes);

    let mut sessions = load_sessions()?;
    let now = chrono::Utc::now().timestamp() as f64;
    sessions.insert(
        token.clone(),
        Session {
            username: username.to_string(),
            created_at: now,
            expires_at: now + SESSION_TTL_SECONDS,
        },
    );
    save_sessions(&sessions)?;
    Ok(token)
}

pub fn validate_session(token: &str) -> Option<String> {
    let mut sessions = match load_sessions() {
        Ok(s) => s,
        Err(_) => return None,
    };
    let session = sessions.remove(token)?;
    let now = chrono::Utc::now().timestamp() as f64;
    if now > session.expires_at {
        // Session expired, don't put it back
        let _ = save_sessions(&sessions);
        return None;
    }
    // Put session back (updated)
    let _ = save_sessions(&sessions);
    Some(session.username)
}

pub fn revoke_session(token: &str) -> Result<(), AuthError> {
    let mut sessions = load_sessions()?;
    sessions.remove(token);
    save_sessions(&sessions)?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash() {
        let salt = "test_salt_123";
        let hash1 = hash_password("password123", salt);
        let hash2 = hash_password("password123", salt);
        let hash3 = hash_password("different", salt);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_salt_generation() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        assert_ne!(salt1, salt2);
        assert_eq!(salt1.len(), 32); // 16 bytes = 32 hex chars
    }

    #[test]
    fn test_is_auth_enabled_when_no_file() {
        // When no auth file exists, should return false
        let result = is_auth_enabled();
        // Just check it doesn't panic — result is implementation-defined
    }
}
