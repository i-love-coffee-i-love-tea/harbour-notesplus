//! Authentication module for Notes++ Embedded Server:
//! Enforces verification code authorization and manages sessions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use crate::error::NotesError;

pub fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Generates a cryptographically secure random hex string of given byte length.
pub fn generate_secure_token(byte_len: usize) -> Result<String, NotesError> {
    let rng = SystemRandom::new();
    let mut bytes = vec![0u8; byte_len];
    rng.fill(&mut bytes).map_err(|e| NotesError::Auth(format!("Failed to generate random bytes: {}", e)))?;
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Generates a short numeric verification code (e.g. "4729") for challenge confirmation.
pub fn generate_verification_code(digit_count: u32) -> String {
    let rng = SystemRandom::new();
    let max = 10u32.pow(digit_count);
    let mut bytes = [0u8; 4];
    rng.fill(&mut bytes).expect("CSPRNG failure — cannot generate secure verification code");
    let num = u32::from_be_bytes(bytes) % max;
    format!("{:0width$}", num, width = digit_count as usize)
}

/// Constant-time comparison between two string slices to prevent timing attacks.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Authentication configuration stored in server state / persistent settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub session_expiry_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            session_expiry_secs: crate::constants::SESSION_EXPIRY_SECS,
        }
    }
}

/// Active user session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user: String,
    pub auth_method: String,
    pub created_at: u64,
    pub expires_at: u64,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        current_epoch_secs() >= self.expires_at
    }
}

/// Thread-safe session store with optional file-backed persistence.
#[derive(Clone, Default)]
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    storage_path: Option<PathBuf>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            storage_path: None,
        }
    }

    /// Creates a session store that persists active sessions to `path`.
    pub fn with_storage(path: PathBuf) -> Self {
        let sessions = if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(loaded) = serde_json::from_str::<HashMap<String, Session>>(&data) {
                    let now = current_epoch_secs();
                    loaded
                        .into_iter()
                        .filter(|(_, s)| s.expires_at > now)
                        .collect()
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        Self {
            sessions: Arc::new(Mutex::new(sessions)),
            storage_path: Some(path),
        }
    }

    fn persist(&self, map: &HashMap<String, Session>) {
        if let Some(ref path) = self.storage_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(map) {
                let _ = std::fs::write(path, json.as_bytes());
            }
        }
    }

    /// Validates a session token. Returns the session if valid and not expired.
    pub fn validate_session(&self, token: &str) -> Option<Session> {
        if token.trim().is_empty() {
            return None;
        }
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let now = current_epoch_secs();

        // Prune expired
        let mut changed = false;
        map.retain(|_, s| {
            let valid = s.expires_at > now;
            if !valid {
                changed = true;
            }
            valid
        });
        if changed {
            self.persist(&map);
        }

        let mut matched = None;
        for (id, sess) in map.iter() {
            if constant_time_eq(id, token) {
                matched = Some(sess.clone());
            }
        }
        matched
    }

    /// Creates and stores a new active session for the given username and auth method.
    pub fn create_session(
        &self,
        username: &str,
        auth_method: &str,
        ttl_secs: u64,
    ) -> Result<Session, NotesError> {
        let now = current_epoch_secs();
        let session_id = generate_secure_token(32)?;

        let session = Session {
            id: session_id.clone(),
            user: username.to_string(),
            auth_method: auth_method.to_string(),
            created_at: now,
            expires_at: now + ttl_secs,
        };

        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(session_id, session.clone());
        self.persist(&map);

        Ok(session)
    }

    /// Revokes and removes a session by ID.
    pub fn remove_session(&self, session_id: &str) {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if map.remove(session_id).is_some() {
            self.persist(&map);
        }
    }

    /// Cleans up all expired sessions from memory and disk.
    pub fn clean_expired(&self) {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let now = current_epoch_secs();
        let initial_len = map.len();
        map.retain(|_, s| s.expires_at > now);
        if map.len() != initial_len {
            self.persist(&map);
        }
    }
}

/// Status of an authorization challenge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChallengeStatus {
    Pending,
    Approved,
    Denied,
}

/// A pending authorization challenge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub challenge_id: String,
    pub verification_code: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: ChallengeStatus,
}

impl AuthChallenge {
    pub fn is_expired(&self) -> bool {
        current_epoch_secs() >= self.expires_at
    }
}

/// Thread-safe store for authorization challenges.
#[derive(Clone, Default)]
pub struct AuthChallengeStore {
    challenges: Arc<Mutex<HashMap<String, AuthChallenge>>>,
}

impl AuthChallengeStore {
    pub fn new() -> Self {
        Self {
            challenges: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Creates a new pending challenge with the given TTL in seconds.
    pub fn create_challenge(&self, ttl_secs: u64) -> Result<AuthChallenge, NotesError> {
        let now = current_epoch_secs();
        let challenge = AuthChallenge {
            challenge_id: generate_secure_token(32)?,
            verification_code: generate_verification_code(4),
            created_at: now,
            expires_at: now + ttl_secs,
            status: ChallengeStatus::Pending,
        };
        let mut map = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(challenge.challenge_id.clone(), challenge.clone());
        Ok(challenge)
    }

    /// Returns a challenge if it exists and hasn't been cleaned up.
    pub fn get_challenge(&self, id: &str) -> Option<AuthChallenge> {
        let mut map = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        map.retain(|_, c| !c.is_expired() || c.status == ChallengeStatus::Approved);
        map.get(id).cloned()
    }

    /// Marks a pending challenge as approved. Returns false if not found or not pending.
    pub fn approve_challenge(&self, id: &str) -> bool {
        let mut map = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(challenge) = map.get_mut(id) {
            if challenge.status == ChallengeStatus::Pending && !challenge.is_expired() {
                challenge.status = ChallengeStatus::Approved;
                return true;
            }
        }
        false
    }

    /// Marks a pending challenge as denied. Returns false if not found or not pending.
    pub fn deny_challenge(&self, id: &str) -> bool {
        let mut map = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(challenge) = map.get_mut(id) {
            if challenge.status == ChallengeStatus::Pending {
                challenge.status = ChallengeStatus::Denied;
                return true;
            }
        }
        false
    }

    /// Removes a challenge from the store (e.g. after browser has consumed the result).
    pub fn remove_challenge(&self, id: &str) {
        let mut map = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_store_lifecycle() {
        let store = SessionStore::new();
        let session = store.create_session("alice", "code", 3600).unwrap();
        assert_eq!(session.user, "alice");

        let validated = store.validate_session(&session.id);
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().user, "alice");

        store.remove_session(&session.id);
        assert!(store.validate_session(&session.id).is_none());
    }

    #[test]
    fn test_session_store_persistence() {
        let tmp_dir = std::env::temp_dir().join(format!("notes_sess_test_{}", generate_verification_code(6)));
        let sess_file = tmp_dir.join("sessions.json");

        {
            let store = SessionStore::with_storage(sess_file.clone());
            let _ = store.create_session("bob", "code", 3600).unwrap();
        }

        // Reopen from disk
        let store2 = SessionStore::with_storage(sess_file.clone());
        let map = store2.sessions.lock().unwrap().clone();
        assert_eq!(map.len(), 1);
        assert_eq!(map.values().next().unwrap().user, "bob");

        let _ = std::fs::remove_dir_all(tmp_dir);
    }

    #[test]
    fn test_verification_code_generation() {
        let code = generate_verification_code(4);
        assert_eq!(code.len(), 4);
        assert!(code.chars().all(|c| c.is_ascii_digit()));

        let code6 = generate_verification_code(6);
        assert_eq!(code6.len(), 6);
        assert!(code6.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_auth_challenge_lifecycle() {
        let store = AuthChallengeStore::new();
        let challenge = store.create_challenge(60).unwrap();
        assert_eq!(challenge.status, ChallengeStatus::Pending);
        assert_eq!(challenge.verification_code.len(), 4);

        let retrieved = store.get_challenge(&challenge.challenge_id);
        assert!(retrieved.is_some());

        let approved = store.approve_challenge(&challenge.challenge_id);
        assert!(approved);

        let after_approval = store.get_challenge(&challenge.challenge_id).unwrap();
        assert_eq!(after_approval.status, ChallengeStatus::Approved);
    }

    #[test]
    fn test_auth_challenge_deny() {
        let store = AuthChallengeStore::new();
        let challenge = store.create_challenge(60).unwrap();
        let denied = store.deny_challenge(&challenge.challenge_id);
        assert!(denied);

        let after_deny = store.get_challenge(&challenge.challenge_id).unwrap();
        assert_eq!(after_deny.status, ChallengeStatus::Denied);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("secret123", "secret123"));
        assert!(!constant_time_eq("secret123", "secret124"));
        assert!(!constant_time_eq("secret123", "secret12"));
        assert!(!constant_time_eq("secret12", "secret123"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn test_auth_challenge_remove() {
        let store = AuthChallengeStore::new();
        let challenge = store.create_challenge(60).unwrap();
        store.remove_challenge(&challenge.challenge_id);
        assert!(store.get_challenge(&challenge.challenge_id).is_none());
    }
}
