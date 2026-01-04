use serde::{Deserialize, Serialize};

// === Request Types ===

#[derive(Debug, Serialize)]
pub struct CreateDocumentRequest {
    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_clean_html: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_using: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateDocumentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seen: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Default)]
pub struct ListDocumentsParams {
    pub id: Option<String>,
    pub updated_after: Option<String>,
    pub location: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub page_cursor: Option<String>,
    pub with_html_content: Option<bool>,
    pub with_raw_source_url: Option<bool>,
}

// === Response Types ===

#[derive(Debug, Deserialize, Serialize)]
pub struct Document {
    pub id: String,
    pub url: String,
    pub source_url: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub category: String,
    pub location: String,
    pub tags: Option<serde_json::Value>,
    pub site_name: Option<String>,
    pub word_count: Option<u32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub published_date: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub content: Option<String>,
    pub html_content: Option<String>,
    pub parent_id: Option<String>,
    pub reading_progress: Option<f32>,
    pub first_opened_at: Option<String>,
    pub last_opened_at: Option<String>,
    pub saved_at: Option<String>,
    pub last_moved_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListDocumentsResponse {
    pub count: u32,
    #[serde(rename = "nextPageCursor")]
    pub next_page_cursor: Option<String>,
    pub results: Vec<Document>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateDocumentResponse {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListTagsResponse {
    pub count: u32,
    #[serde(rename = "nextPageCursor")]
    pub next_page_cursor: Option<String>,
    pub results: Vec<Tag>,
}

