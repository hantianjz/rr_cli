use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::error::RrCliError;
use crate::types::*;

const MAX_REQUESTS_PER_SESSION: u32 = 20;
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
            timestamp: Utc::now().to_rfc3339(),
            method: method.to_string(),
            url: url.to_string(),
            request_body: request_body.and_then(|b| serde_json::from_str(b).ok()),
            status,
            response_body: response_body.and_then(|b| serde_json::from_str(b).ok()),
        };
        self.entries.push(entry);
    }

    pub fn save(&self) -> Result<(), RrCliError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(DEBUG_CACHE_FILE, content)?;
        Ok(())
    }
}

pub struct ReaderClient {
    client: reqwest::Client,
    request_count: u32,
    verbose: bool,
    debug_cache: Option<DebugCache>,
}

impl ReaderClient {
    pub fn new(token: &str, verbose: bool) -> Result<Self, RrCliError> {
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
            request_count: 0,
            verbose,
            debug_cache,
        })
    }

    pub fn save_debug_cache(&self) -> Result<(), RrCliError> {
        if let Some(cache) = &self.debug_cache {
            cache.save()?;
            if self.verbose {
                eprintln!("[DEBUG] Saved debug cache to {}", DEBUG_CACHE_FILE);
            }
        }
        Ok(())
    }

    fn check_rate_limit(&mut self) -> Result<(), RrCliError> {
        if self.request_count >= MAX_REQUESTS_PER_SESSION {
            return Err(RrCliError::RateLimitExceeded(MAX_REQUESTS_PER_SESSION));
        }
        self.request_count += 1;
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

    fn log_response(&mut self, method: &str, url: &str, request_body: Option<&str>, status: StatusCode, response_body: &str) {
        if self.verbose {
            eprintln!("[DEBUG] <-- {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));
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
                if response_body.is_empty() { None } else { Some(response_body) },
            );
        }
    }

    async fn handle_error_response(&self, response: reqwest::Response) -> RrCliError {
        let status = response.status();

        if status == StatusCode::UNAUTHORIZED {
            return RrCliError::AuthError;
        }

        if status == StatusCode::NOT_FOUND {
            return RrCliError::NotFound("Resource not found".to_string());
        }

        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            return RrCliError::ApiRateLimited(retry_after);
        }

        let error_text = response.text().await.unwrap_or_default();
        RrCliError::ApiError(format!("HTTP {}: {}", status, error_text))
    }

    pub async fn check_auth(&mut self) -> Result<bool, RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v2/auth/", BASE_URL);
        self.log_request("GET", &url, None);

        let response = self.client.get(&url).send().await?;
        let status = response.status();

        self.log_response("GET", &url, None, status, "");

        if status == StatusCode::NO_CONTENT {
            Ok(true)
        } else if status == StatusCode::UNAUTHORIZED {
            Ok(false)
        } else {
            Err(self.handle_error_response(response).await)
        }
    }

    pub async fn create_document(
        &mut self,
        request: CreateDocumentRequest,
    ) -> Result<CreateDocumentResponse, RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v3/save/", BASE_URL);
        let body = serde_json::to_string(&request).unwrap_or_default();
        self.log_request("POST", &url, Some(&body));

        let response = self.client.post(&url).json(&request).send().await?;
        let status = response.status();

        if status.is_success() {
            let text = response.text().await?;
            self.log_response("POST", &url, Some(&body), status, &text);
            let result = serde_json::from_str(&text)?;
            Ok(result)
        } else {
            let text = response.text().await.unwrap_or_default();
            self.log_response("POST", &url, Some(&body), status, &text);
            Err(RrCliError::ApiError(format!("HTTP {}: {}", status, text)))
        }
    }

    pub async fn list_documents(
        &mut self,
        params: ListDocumentsParams,
    ) -> Result<ListDocumentsResponse, RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v3/list/", BASE_URL);
        let mut request = self.client.get(&url);

        let mut query_params = vec![];

        if let Some(id) = &params.id {
            request = request.query(&[("id", id)]);
            query_params.push(format!("id={}", id));
        }
        if let Some(updated_after) = &params.updated_after {
            request = request.query(&[("updatedAfter", updated_after)]);
            query_params.push(format!("updatedAfter={}", updated_after));
        }
        if let Some(location) = &params.location {
            request = request.query(&[("location", location)]);
            query_params.push(format!("location={}", location));
        }
        if let Some(category) = &params.category {
            request = request.query(&[("category", category)]);
            query_params.push(format!("category={}", category));
        }
        if let Some(tag) = &params.tag {
            request = request.query(&[("tag", tag)]);
            query_params.push(format!("tag={}", tag));
        }
        if let Some(cursor) = &params.page_cursor {
            request = request.query(&[("pageCursor", cursor)]);
            query_params.push(format!("pageCursor={}", cursor));
        }
        if let Some(with_html) = params.with_html_content {
            request = request.query(&[("withHtmlContent", with_html.to_string())]);
            query_params.push(format!("withHtmlContent={}", with_html));
        }
        if let Some(with_raw) = params.with_raw_source_url {
            request = request.query(&[("withRawSourceUrl", with_raw.to_string())]);
            query_params.push(format!("withRawSourceUrl={}", with_raw));
        }

        let full_url = if query_params.is_empty() {
            url
        } else {
            format!("{}?{}", url, query_params.join("&"))
        };
        self.log_request("GET", &full_url, None);

        let response = request.send().await?;
        let status = response.status();

        if status.is_success() {
            let text = response.text().await?;
            self.log_response("GET", &full_url, None, status, &text);
            let result = serde_json::from_str(&text)?;
            Ok(result)
        } else {
            let text = response.text().await.unwrap_or_default();
            self.log_response("GET", &full_url, None, status, &text);
            Err(RrCliError::ApiError(format!("HTTP {}: {}", status, text)))
        }
    }

    pub async fn update_document(
        &mut self,
        id: &str,
        request: UpdateDocumentRequest,
    ) -> Result<Document, RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v3/update/{}/", BASE_URL, id);
        let body = serde_json::to_string(&request).unwrap_or_default();
        self.log_request("PATCH", &url, Some(&body));

        let response = self.client.patch(&url).json(&request).send().await?;
        let status = response.status();

        if status.is_success() {
            let text = response.text().await?;
            self.log_response("PATCH", &url, Some(&body), status, &text);
            let result = serde_json::from_str(&text)?;
            Ok(result)
        } else {
            let text = response.text().await.unwrap_or_default();
            self.log_response("PATCH", &url, Some(&body), status, &text);
            Err(RrCliError::ApiError(format!("HTTP {}: {}", status, text)))
        }
    }

    pub async fn delete_document(&mut self, id: &str) -> Result<(), RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v3/delete/{}/", BASE_URL, id);
        self.log_request("DELETE", &url, None);

        let response = self.client.delete(&url).send().await?;
        let status = response.status();

        self.log_response("DELETE", &url, None, status, "");

        if status == StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(self.handle_error_response(response).await)
        }
    }

    pub async fn list_all_tags(&mut self) -> Result<Vec<String>, RrCliError> {
        let mut all_tags = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            self.check_rate_limit()?;

            let url = format!("{}/v3/tags/", BASE_URL);
            let mut request = self.client.get(&url);

            let full_url = if let Some(c) = &cursor {
                request = request.query(&[("pageCursor", c)]);
                format!("{}?pageCursor={}", url, c)
            } else {
                url.clone()
            };

            self.log_request("GET", &full_url, None);

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let text = response.text().await.unwrap_or_default();
                self.log_response("GET", &full_url, None, status, &text);
                return Err(RrCliError::ApiError(format!("HTTP {}: {}", status, text)));
            }

            let text = response.text().await?;
            self.log_response("GET", &full_url, None, status, &text);

            let result: ListTagsResponse = serde_json::from_str(&text)?;

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
