use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::StatusCode;

use crate::error::RrCliError;
use crate::types::*;

const MAX_REQUESTS_PER_SESSION: u32 = 20;
const BASE_URL: &str = "https://readwise.io/api";

pub struct ReaderClient {
    client: reqwest::Client,
    request_count: u32,
}

impl ReaderClient {
    pub fn new(token: &str) -> Result<Self, RrCliError> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Token {}", token);
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            request_count: 0,
        })
    }

    fn check_rate_limit(&mut self) -> Result<(), RrCliError> {
        if self.request_count >= MAX_REQUESTS_PER_SESSION {
            return Err(RrCliError::RateLimitExceeded(MAX_REQUESTS_PER_SESSION));
        }
        self.request_count += 1;
        Ok(())
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
        let response = self.client.get(&url).send().await?;

        if response.status() == StatusCode::NO_CONTENT {
            Ok(true)
        } else if response.status() == StatusCode::UNAUTHORIZED {
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
        let response = self.client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            let result = response.json().await?;
            Ok(result)
        } else {
            Err(self.handle_error_response(response).await)
        }
    }

    pub async fn list_documents(
        &mut self,
        params: ListDocumentsParams,
    ) -> Result<ListDocumentsResponse, RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v3/list/", BASE_URL);
        let mut request = self.client.get(&url);

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

        let response = request.send().await?;

        if response.status().is_success() {
            let result = response.json().await?;
            Ok(result)
        } else {
            Err(self.handle_error_response(response).await)
        }
    }

    pub async fn update_document(
        &mut self,
        id: &str,
        request: UpdateDocumentRequest,
    ) -> Result<Document, RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v3/update/{}/", BASE_URL, id);
        let response = self.client.patch(&url).json(&request).send().await?;

        if response.status().is_success() {
            let result = response.json().await?;
            Ok(result)
        } else {
            Err(self.handle_error_response(response).await)
        }
    }

    pub async fn delete_document(&mut self, id: &str) -> Result<(), RrCliError> {
        self.check_rate_limit()?;

        let url = format!("{}/v3/delete/{}/", BASE_URL, id);
        let response = self.client.delete(&url).send().await?;

        if response.status() == StatusCode::NO_CONTENT {
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

            if let Some(c) = &cursor {
                request = request.query(&[("pageCursor", c)]);
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                return Err(self.handle_error_response(response).await);
            }

            let result: ListTagsResponse = response.json().await?;

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
