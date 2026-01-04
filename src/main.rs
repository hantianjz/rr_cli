mod cache;
mod cli;
mod client;
mod error;
mod output;
mod types;

use clap::Parser;

use cache::Cache;
use cli::{Args, Command, CreateArgs, ListArgs, UpdateArgs};
use client::ReaderClient;
use error::RrCliError;
use types::*;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(e) = run(args).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<(), RrCliError> {
    let token = args.token.ok_or_else(|| {
        RrCliError::ApiError(
            "Missing API token. Set READWISE_ACCESS_TOKEN env var or use --token".to_string(),
        )
    })?;

    let mut client = ReaderClient::new(&token)?;
    let mut cache = if args.cache {
        Some(Cache::new(&args.cache_file))
    } else {
        None
    };

    let result = match args.command {
        Command::Auth => handle_auth(&mut client, args.json).await,
        Command::Create(create_args) => {
            handle_create(&mut client, create_args, args.json).await
        }
        Command::List(list_args) => {
            handle_list(&mut client, list_args, args.json, &mut cache).await
        }
        Command::Update(update_args) => {
            handle_update(&mut client, update_args, args.json).await
        }
        Command::Delete(delete_args) => {
            handle_delete(&mut client, &delete_args.id, args.json).await
        }
        Command::TagList => handle_tag_list(&mut client, args.json, &mut cache).await,
    };

    // Save cache if enabled
    if let Some(c) = cache {
        c.save()?;
    }

    result
}

async fn handle_auth(client: &mut ReaderClient, json_output: bool) -> Result<(), RrCliError> {
    let is_valid = client.check_auth().await?;
    let output = if is_valid {
        output::format_auth_success(json_output)
    } else {
        output::format_auth_failure(json_output)
    };
    println!("{}", output);
    Ok(())
}

async fn handle_create(
    client: &mut ReaderClient,
    args: CreateArgs,
    json_output: bool,
) -> Result<(), RrCliError> {
    let request = CreateDocumentRequest {
        url: args.url,
        html: args.html,
        should_clean_html: args.should_clean_html,
        title: args.title,
        author: args.author,
        summary: args.summary,
        published_date: args.published_date,
        image_url: args.image_url,
        location: args.location.map(|l| l.as_str().to_string()),
        category: args.category.map(|c| c.as_str().to_string()),
        saved_using: args.saved_using,
        tags: args.tags,
        notes: args.notes,
    };

    let response = client.create_document(request).await?;
    println!("{}", output::format_create_response(&response, json_output));
    Ok(())
}

async fn handle_list(
    client: &mut ReaderClient,
    args: ListArgs,
    json_output: bool,
    cache: &mut Option<Cache>,
) -> Result<(), RrCliError> {
    let params = ListDocumentsParams {
        id: args.id,
        updated_after: args.updated_after,
        location: args.location.map(|l| l.as_str().to_string()),
        category: args.category.map(|c| c.as_str().to_string()),
        tag: args.tag,
        page_cursor: args.cursor,
        with_html_content: args.with_html_content,
        with_raw_source_url: args.with_raw_source_url,
    };

    // Check cache first
    let cache_key = if cache.is_some() {
        let params_json = serde_json::json!({
            "id": params.id,
            "updated_after": params.updated_after,
            "location": params.location,
            "category": params.category,
            "tag": params.tag,
            "page_cursor": params.page_cursor,
            "with_html_content": params.with_html_content,
            "with_raw_source_url": params.with_raw_source_url,
        });
        Some(Cache::generate_key("list", &params_json))
    } else {
        None
    };

    if let (Some(c), Some(key)) = (cache.as_ref(), &cache_key) {
        if let Some(entry) = c.get(key) {
            if let Ok(response) = serde_json::from_value::<ListDocumentsResponse>(entry.response.clone()) {
                println!("{}", output::format_list_response(&response, json_output));
                return Ok(());
            }
        }
    }

    let response = client.list_documents(params).await?;

    // Store in cache
    if let (Some(c), Some(key)) = (cache.as_mut(), cache_key) {
        let params_json = serde_json::json!({});
        let response_json = serde_json::to_value(&response)?;
        c.set(&key, "list", params_json, response_json);
    }

    println!("{}", output::format_list_response(&response, json_output));
    Ok(())
}

async fn handle_update(
    client: &mut ReaderClient,
    args: UpdateArgs,
    json_output: bool,
) -> Result<(), RrCliError> {
    let request = UpdateDocumentRequest {
        title: args.title,
        author: args.author,
        summary: args.summary,
        published_date: args.published_date,
        image_url: args.image_url,
        seen: args.seen,
        location: args.location.map(|l| l.as_str().to_string()),
        category: args.category.map(|c| c.as_str().to_string()),
        tags: args.tags,
    };

    let response = client.update_document(&args.id, request).await?;
    println!("{}", output::format_update_response(&response, json_output));
    Ok(())
}

async fn handle_delete(
    client: &mut ReaderClient,
    id: &str,
    json_output: bool,
) -> Result<(), RrCliError> {
    client.delete_document(id).await?;
    println!("{}", output::format_delete_response(id, json_output));
    Ok(())
}

async fn handle_tag_list(
    client: &mut ReaderClient,
    json_output: bool,
    cache: &mut Option<Cache>,
) -> Result<(), RrCliError> {
    let cache_key = "tag_list:all";

    // Check cache first
    if let Some(c) = cache.as_ref() {
        if let Some(entry) = c.get(cache_key) {
            if let Ok(tags) = serde_json::from_value::<Vec<String>>(entry.response.clone()) {
                println!("{}", output::format_tags_response(&tags, json_output));
                return Ok(());
            }
        }
    }

    let tags = client.list_all_tags().await?;

    // Store in cache
    if let Some(c) = cache.as_mut() {
        let response_json = serde_json::to_value(&tags)?;
        c.set(cache_key, "tag_list", serde_json::json!({}), response_json);
    }

    println!("{}", output::format_tags_response(&tags, json_output));
    Ok(())
}
