//! Origin allowlist + per-origin pairing tokens, per docs/PROTOCOL.md's Auth
//! section. Tokens are persisted to disk so a re-paired app doesn't need the
//! user to click "allow" again after an agent restart.

use rand::distr::Alphanumeric;
use rand::RngExt;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Reads the allowlist from `PRINTERS_AGENT_ORIGINS` (comma-separated), falling
/// back to a small localhost-only default so the agent is usable out of the box
/// during development.
pub fn allowed_origins() -> Vec<String> {
    match std::env::var("PRINTERS_AGENT_ORIGINS") {
        Ok(v) => v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        Err(_) => vec!["http://localhost".to_string(), "https://localhost".to_string()],
    }
}

pub fn origin_allowed(allowlist: &[String], origin: &str) -> bool {
    allowlist.iter().any(|allowed| origin == allowed || origin.starts_with(&format!("{allowed}:")))
}

fn tokens_path() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("printers-agent");
    let _ = fs::create_dir_all(&dir);
    dir.join("tokens.json")
}

pub struct TokenStore {
    /// `None` means "don't touch disk" -- used for tests, so concurrent test
    /// runs never race on the same real `tokens.json`.
    path: Option<PathBuf>,
    tokens: Mutex<HashMap<String, String>>,
}

impl TokenStore {
    pub fn load() -> Self {
        let path = tokens_path();
        let tokens = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path: Some(path), tokens: Mutex::new(tokens) }
    }

    /// A TokenStore that never touches disk -- for tests and any other
    /// in-process-only use.
    pub fn in_memory() -> Self {
        Self { path: None, tokens: Mutex::new(HashMap::new()) }
    }

    fn save(&self, tokens: &HashMap<String, String>) {
        let Some(path) = &self.path else { return };
        if let Ok(json) = serde_json::to_string_pretty(tokens) {
            let _ = fs::write(path, json);
        }
    }

    /// Issues (or reissues) a token for `origin` and persists it.
    pub fn pair(&self, origin: &str) -> String {
        let token: String = rand::rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
        let mut tokens = self.tokens.lock().unwrap();
        tokens.insert(origin.to_string(), token.clone());
        self.save(&tokens);
        token
    }

    pub fn is_valid(&self, origin: &str, token: &str) -> bool {
        self.tokens.lock().unwrap().get(origin).is_some_and(|t| t == token)
    }

    pub fn revoke(&self, origin: &str) {
        let mut tokens = self.tokens.lock().unwrap();
        tokens.remove(origin);
        self.save(&tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_allowed_matches_exact_and_with_port() {
        let allowlist = vec!["http://localhost:5173".to_string(), "https://app.example.com".to_string()];
        assert!(origin_allowed(&allowlist, "http://localhost:5173"));
        assert!(!origin_allowed(&allowlist, "http://evil.com"));
        assert!(origin_allowed(&allowlist, "https://app.example.com"));
    }

    #[test]
    fn pair_then_validate_round_trips() {
        let store = TokenStore::in_memory();
        let token = store.pair("http://localhost:5173");
        assert!(store.is_valid("http://localhost:5173", &token));
        assert!(!store.is_valid("http://localhost:5173", "wrong-token"));
        store.revoke("http://localhost:5173");
        assert!(!store.is_valid("http://localhost:5173", &token));
    }
}
