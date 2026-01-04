use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::RrCliError;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub timestamp: u64,
    pub endpoint: String,
    pub params: serde_json::Value,
    pub response: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CacheFile {
    pub entries: HashMap<String, CacheEntry>,
}

pub struct Cache {
    file_path: String,
    data: CacheFile,
}

impl Cache {
    pub fn new(file_path: &str) -> Self {
        let data = Self::load_from_file(file_path).unwrap_or_default();
        Self {
            file_path: file_path.to_string(),
            data,
        }
    }

    fn load_from_file(file_path: &str) -> Option<CacheFile> {
        let path = Path::new(file_path);
        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<(), RrCliError> {
        let content = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.data.entries.get(key)
    }

    pub fn set(
        &mut self,
        key: &str,
        endpoint: &str,
        params: serde_json::Value,
        response: serde_json::Value,
    ) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = CacheEntry {
            timestamp,
            endpoint: endpoint.to_string(),
            params,
            response,
        };
        self.data.entries.insert(key.to_string(), entry);
    }

    pub fn generate_key(endpoint: &str, params: &serde_json::Value) -> String {
        let params_str = serde_json::to_string(params).unwrap_or_default();
        format!("{}:{}", endpoint, params_str)
    }
}
