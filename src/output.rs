use crate::types::*;

pub fn format_auth_success(json_output: bool) -> String {
    if json_output {
        r#"{"authenticated": true}"#.to_string()
    } else {
        "Authentication: valid".to_string()
    }
}

pub fn format_auth_failure(json_output: bool) -> String {
    if json_output {
        r#"{"authenticated": false}"#.to_string()
    } else {
        "Authentication: invalid or expired token".to_string()
    }
}

pub fn format_create_response(response: &CreateDocumentResponse, json_output: bool) -> String {
    if json_output {
        serde_json::to_string(response).unwrap_or_default()
    } else {
        format!(
            "Document created\n  ID: {}\n  URL: {}",
            response.id, response.url
        )
    }
}

pub fn format_list_response(response: &ListDocumentsResponse, json_output: bool) -> String {
    if json_output {
        serde_json::to_string(response).unwrap_or_default()
    } else {
        let mut output = format!("Documents: {} total\n", response.count);

        for doc in &response.results {
            output.push_str(&format_document(doc));
            output.push('\n');
        }

        if let Some(cursor) = &response.next_page_cursor {
            output.push_str(&format!("Next cursor: {}\n", cursor));
        }

        output
    }
}

fn format_document(doc: &Document) -> String {
    let title = doc.title.as_deref().unwrap_or("(no title)");
    let mut output = format!("Document: {}\n", title);
    output.push_str(&format!("  ID: {}\n", doc.id));

    if let Some(location) = &doc.location {
        output.push_str(&format!("  Location: {}\n", location));
    }
    if let Some(category) = &doc.category {
        output.push_str(&format!("  Category: {}\n", category));
    }
    if let Some(author) = &doc.author {
        output.push_str(&format!("  Author: {}\n", author));
    }

    if let Some(tags) = &doc.tags {
        let tags_str = format_tags(tags);
        if !tags_str.is_empty() {
            output.push_str(&format!("  Tags: {}\n", tags_str));
        }
    }

    output
}

fn format_tags(tags: &serde_json::Value) -> String {
    match tags {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>()
            .join(", "),
        serde_json::Value::Object(obj) => obj.keys().cloned().collect::<Vec<_>>().join(", "),
        _ => String::new(),
    }
}

pub fn format_update_response(doc: &Document, json_output: bool) -> String {
    if json_output {
        serde_json::to_string(doc).unwrap_or_default()
    } else {
        let mut output = "Document updated\n".to_string();
        output.push_str(&format_document(doc));
        output
    }
}

pub fn format_delete_response(id: &str, json_output: bool) -> String {
    if json_output {
        format!(r#"{{"deleted": true, "id": "{}"}}"#, id)
    } else {
        format!("Document deleted: {}", id)
    }
}

pub fn format_tags_response(tags: &[Tag], json_output: bool) -> String {
    if json_output {
        serde_json::to_string(tags).unwrap_or_default()
    } else {
        tags.iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
