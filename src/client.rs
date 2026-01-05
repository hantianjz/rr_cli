use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

use anyhow::{anyhow, Result};

use crate::types::*;

/// Parse retry seconds from API error response body
/// Looks for pattern like "Expected available in X seconds"
fn parse_retry_seconds(body: &str) -> Option<u64> {
    // Try to parse JSON response first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(detail) = json.get("detail").and_then(|d| d.as_str()) {
            return extract_seconds_from_message(detail);
        }
    }
    // Fallback: try to extract from raw text
    extract_seconds_from_message(body)
}

fn extract_seconds_from_message(msg: &str) -> Option<u64> {
    // Look for "in X second" pattern
    let patterns = ["in ", "In "];
    for pattern in patterns {
        if let Some(idx) = msg.find(pattern) {
            let after = &msg[idx + pattern.len()..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(secs) = num_str.parse::<u64>() {
                return Some(secs);
            }
        }
    }
    None
}

/// Display countdown on same line and wait
async fn countdown_wait(seconds: u64) {
    for remaining in (1..=seconds).rev() {
        eprint!("\rRate limited. Retrying in {} seconds...  ", remaining);
        std::io::stderr().flush().ok();
        sleep(Duration::from_secs(1)).await;
    }
    eprint!("\rRetrying now...                           \n");
    std::io::stderr().flush().ok();
}

const BASE_URL: &str = "https://readwise.io/api";
const DEBUG_CACHE_FILE: &str = "debug_cache.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct DebugEntry {
    pub timestamp: String,
    pub method: String,
    pub url: String,
    pub request_body: Option<serde_json::Value>,
    pub status: u16,
    pub response_body: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DebugCache {
    pub entries: Vec<DebugEntry>,
}

impl DebugCache {
    pub fn new() -> Self {
        // Load existing entries from file if it exists
        if let Ok(content) = fs::read_to_string(DEBUG_CACHE_FILE) {
            if let Ok(cache) = serde_json::from_str::<DebugCache>(&content) {
                return cache;
            }
        }
        Self { entries: vec![] }
    }

    pub fn add_entry(
        &mut self,
        method: &str,
        url: &str,
        request_body: Option<&str>,
        status: u16,
        response_body: Option<&str>,
    ) {
        let entry = DebugEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default(),
            method: method.to_string(),
            url: url.to_string(),
            request_body: request_body.and_then(|b| serde_json::from_str(b).ok()),
            status,
            response_body: response_body.and_then(|b| serde_json::from_str(b).ok()),
        };
        self.entries.push(entry);
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(DEBUG_CACHE_FILE, content)?;
        Ok(())
    }

    /// Try to save the debug cache if the file exists
    /// Used by signal handlers to save cache on interrupt/panic
    pub fn save_if_exists() -> Result<()> {
        let path = Path::new(DEBUG_CACHE_FILE);
        if !path.exists() {
            // No debug cache file exists yet, nothing to save
            return Ok(());
        }

        // Load and save the debug cache to persist any in-memory changes
        let cache = Self::new();
        cache.save()
    }
}

pub struct ReaderClient {
    client: reqwest::Client,
    verbose: bool,
    debug_cache: Option<DebugCache>,
}

impl ReaderClient {
    pub fn new(token: &str, verbose: bool) -> Result<Self> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Token {}", token);
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let debug_cache = if verbose {
            Some(DebugCache::new())
        } else {
            None
        };

        Ok(Self {
            client,
            verbose,
            debug_cache,
        })
    }

    pub fn save_debug_cache(&self) -> Result<()> {
        if let Some(cache) = &self.debug_cache {
            cache.save()?;
            if self.verbose {
                eprintln!("[DEBUG] Saved debug cache to {}", DEBUG_CACHE_FILE);
            }
        }
        Ok(())
    }

    fn log_request(&self, method: &str, url: &str, body: Option<&str>) {
        if self.verbose {
            eprintln!("[DEBUG] --> {} {}", method, url);
            if let Some(b) = body {
                eprintln!("[DEBUG] --> Body: {}", b);
            }
        }
    }

    fn log_response(
        &mut self,
        method: &str,
        url: &str,
        request_body: Option<&str>,
        status: StatusCode,
        response_body: &str,
    ) {
        if self.verbose {
            eprintln!(
                "[DEBUG] <-- {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("")
            );
            if !response_body.is_empty() {
                eprintln!("[DEBUG] <-- Body: {}", response_body);
            }
        }

        if let Some(cache) = &mut self.debug_cache {
            cache.add_entry(
                method,
                url,
                request_body,
                status.as_u16(),
                if response_body.is_empty() {
                    None
                } else {
                    Some(response_body)
                },
            );
        }
    }

    async fn execute_request<T, B, S, P>(
        &mut self,
        method: &str,
        url: &str,
        build_request: B,
        check_success: S,
        parse_response: P,
    ) -> Result<T>
    where
        B: Fn(&reqwest::Client) -> (reqwest::RequestBuilder, Option<String>),
        S: Fn(StatusCode) -> bool,
        P: Fn(StatusCode, String) -> Result<T>,
    {
        loop {
            let (request, request_body) = build_request(&self.client);
            let request_body_ref = request_body.as_deref();

            self.log_request(method, url, request_body_ref);

            let response = request.send().await?;
            let status = response.status();

            if check_success(status) {
                let text = response.text().await?;
                self.log_response(method, url, request_body_ref, status, &text);
                return parse_response(status, text);
            } else if status == StatusCode::TOO_MANY_REQUESTS {
                let text = response.text().await.unwrap_or_default();
                self.log_response(method, url, request_body_ref, status, &text);
                let wait_secs = parse_retry_seconds(&text).unwrap_or(60);
                countdown_wait(wait_secs).await;
            } else {
                let text = response.text().await.unwrap_or_default();
                self.log_response(method, url, request_body_ref, status, &text);
                return Err(anyhow!("API request failed: HTTP {}: {}", status, text));
            }
        }
    }

    pub async fn check_auth(&mut self) -> Result<bool> {
        let url = format!("{}/v2/auth/", BASE_URL);

        self.execute_request(
            "GET",
            &url,
            |client| (client.get(&url), None),
            |status| status == StatusCode::NO_CONTENT || status == StatusCode::UNAUTHORIZED,
            |status, _text| {
                if status == StatusCode::NO_CONTENT {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )
        .await
    }

    pub async fn create_document(
        &mut self,
        request: CreateDocumentRequest,
    ) -> Result<CreateDocumentResponse> {
        let url = format!("{}/v3/save/", BASE_URL);

        self.execute_request(
            "POST",
            &url,
            |client| {
                let body = serde_json::to_string(&request).unwrap_or_default();
                (client.post(&url).json(&request), Some(body))
            },
            |status| status.is_success(),
            |_status, text| Ok(serde_json::from_str(&text)?),
        )
        .await
    }

    pub async fn list_documents(
        &mut self,
        params: &ListDocumentsParams,
    ) -> Result<ListDocumentsResponse> {
        let url = format!("{}/v3/list/", BASE_URL);
        let mut query_params = vec![];

        // Build query string for logging
        if let Some(id) = &params.id {
            query_params.push(format!("id={}", id));
        }
        if let Some(updated_after) = &params.updated_after {
            query_params.push(format!("updatedAfter={}", updated_after));
        }
        if let Some(location) = &params.location {
            query_params.push(format!("location={}", location));
        }
        if let Some(category) = &params.category {
            query_params.push(format!("category={}", category));
        }
        if let Some(tag) = &params.tag {
            query_params.push(format!("tag={}", tag));
        }
        if let Some(cursor) = &params.page_cursor {
            query_params.push(format!("pageCursor={}", cursor));
        }
        if let Some(with_html) = params.with_html_content {
            query_params.push(format!("withHtmlContent={}", with_html));
        }
        if let Some(with_raw) = params.with_raw_source_url {
            query_params.push(format!("withRawSourceUrl={}", with_raw));
        }

        let full_url = if query_params.is_empty() {
            url.clone()
        } else {
            format!("{}?{}", url, query_params.join("&"))
        };

        self.execute_request(
            "GET",
            &full_url,
            |client| {
                let mut request = client.get(&url);
                if let Some(id) = &params.id {
                    request = request.query(&[("id", id)]);
                }
                if let Some(updated_after) = &params.updated_after {
                    request = request.query(&[("updatedAfter", updated_after)]);
                }
                if let Some(location) = &params.location {
                    request = request.query(&[("location", location)]);
                }
                if let Some(category) = &params.category {
                    request = request.query(&[("category", category)]);
                }
                if let Some(tag) = &params.tag {
                    request = request.query(&[("tag", tag)]);
                }
                if let Some(cursor) = &params.page_cursor {
                    request = request.query(&[("pageCursor", cursor)]);
                }
                if let Some(with_html) = params.with_html_content {
                    request = request.query(&[("withHtmlContent", with_html.to_string())]);
                }
                if let Some(with_raw) = params.with_raw_source_url {
                    request = request.query(&[("withRawSourceUrl", with_raw.to_string())]);
                }
                (request, None)
            },
            |status| status.is_success(),
            |_status, text| Ok(serde_json::from_str(&text)?),
        )
        .await
    }

    pub async fn update_document(
        &mut self,
        id: &str,
        request: UpdateDocumentRequest,
    ) -> Result<Document> {
        let url = format!("{}/v3/update/{}/", BASE_URL, id);

        self.execute_request(
            "PATCH",
            &url,
            |client| {
                let body = serde_json::to_string(&request).unwrap_or_default();
                (client.patch(&url).json(&request), Some(body))
            },
            |status| status.is_success(),
            |_status, text| Ok(serde_json::from_str(&text)?),
        )
        .await
    }

    pub async fn delete_document(&mut self, id: &str) -> Result<()> {
        let url = format!("{}/v3/delete/{}/", BASE_URL, id);

        self.execute_request(
            "DELETE",
            &url,
            |client| (client.delete(&url), None),
            |status| status == StatusCode::NO_CONTENT,
            |_status, _text| Ok(()),
        )
        .await
    }

    pub async fn list_all_tags(&mut self) -> Result<Vec<String>> {
        let mut all_tags = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let url = format!("{}/v3/tags/", BASE_URL);
            let full_url = if let Some(c) = &cursor {
                format!("{}?pageCursor={}", url, c)
            } else {
                url.clone()
            };

            let result: ListTagsResponse = self
                .execute_request(
                    "GET",
                    &full_url,
                    |client| {
                        let mut request = client.get(&url);
                        if let Some(c) = &cursor {
                            request = request.query(&[("pageCursor", c)]);
                        }
                        (request, None)
                    },
                    |status| status.is_success(),
                    |_status, text| Ok(serde_json::from_str(&text)?),
                )
                .await?;

            for tag in result.results {
                all_tags.push(tag.name);
            }

            match result.next_page_cursor {
                Some(next_cursor) => cursor = Some(next_cursor),
                None => break,
            }
        }

        Ok(all_tags)
    }
}
