mod cache;
mod cli;
mod client;
mod output;
mod types;

use std::io::{self, Write};
use std::sync::Mutex;

use anyhow::{Context, Result};
use clap::Parser;
use once_cell::sync::Lazy;

use cache::Cache;
use cli::{Args, Command, CreateArgs, ListArgs, UpdateArgs};
use client::{DebugCache, ReaderClient};
use types::*;

// Global state for cache file paths (used by signal handlers and panic hooks)
static CACHE_PATHS: Lazy<Mutex<CachePaths>> = Lazy::new(|| {
    Mutex::new(CachePaths {
        cache_file: None,
        debug_cache_file: None,
    })
});

struct CachePaths {
    cache_file: Option<String>,
    debug_cache_file: Option<String>,
}

impl CachePaths {
    fn save_all(&self) {
        // Best-effort save - don't propagate errors in signal handlers
        if let Some(path) = &self.cache_file {
            let _ = Cache::save_if_exists(path).inspect_err(|e| {
                eprintln!("Warning: Failed to save cache: {}", e);
            });
        }
        let _ = DebugCache::save_if_exists().inspect_err(|e| {
            eprintln!("Warning: Failed to save debug cache: {}", e);
        });
    }
}

#[tokio::main]
async fn main() {
    // Install panic hook first (before anything else)
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Call the default hook first (prints panic info)
        default_hook(info);

        // Try to save caches
        eprintln!("Attempting to save caches before panic exit...");
        if let Ok(paths) = CACHE_PATHS.lock() {
            paths.save_all();
        }
    }));

    // Install Ctrl-C handler
    ctrlc::set_handler(move || {
        eprintln!("\nReceived interrupt signal. Saving caches...");
        if let Ok(paths) = CACHE_PATHS.lock() {
            paths.save_all();
        }
        eprintln!("Caches saved. Exiting.");
        std::process::exit(130); // Standard exit code for SIGINT
    })
    .expect("Error setting Ctrl-C handler");

    let args = Args::parse();

    if let Err(e) = run(args).await {
        eprintln!("{}", e);

        // Save caches on error exit
        if let Ok(paths) = CACHE_PATHS.lock() {
            paths.save_all();
        }

        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<()> {
    let token = args
        .token
        .context("Missing API token. Set READWISE_ACCESS_TOKEN env var or use --token")?;

    let mut client = ReaderClient::new(&token, args.verbose)?;

    // Register debug cache path if verbose mode
    if args.verbose {
        if let Ok(mut paths) = CACHE_PATHS.lock() {
            paths.debug_cache_file = Some("debug_cache.json".to_string());
        }
    }

    let mut cache = if args.cache {
        // Register cache path
        if let Ok(mut paths) = CACHE_PATHS.lock() {
            paths.cache_file = Some(args.cache_file.clone());
        }
        Some(Cache::new(&args.cache_file))
    } else {
        None
    };

    let result = match args.command {
        Command::Auth => handle_auth(&mut client, args.json).await,
        Command::Create(create_args) => handle_create(&mut client, create_args, args.json).await,
        Command::List(list_args) => {
            handle_list(&mut client, list_args, args.json, &mut cache).await
        }
        Command::Update(update_args) => handle_update(&mut client, update_args, args.json).await,
        Command::Delete(delete_args) => {
            handle_delete(&mut client, &delete_args.id, args.json).await
        }
        Command::TagList => handle_tag_list(&mut client, args.json, &mut cache).await,
    };

    // Save cache if enabled
    if let Some(c) = cache {
        c.save()?;
    }

    // Save debug cache if verbose mode
    client.save_debug_cache()?;

    result
}

async fn handle_auth(client: &mut ReaderClient, json_output: bool) -> Result<()> {
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
) -> Result<()> {
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
) -> Result<()> {
    let mut params = ListDocumentsParams {
        id: args.id,
        updated_after: args.updated_after,
        location: args.location.map(|l| l.as_str().to_string()),
        category: args.category.map(|c| c.as_str().to_string()),
        tag: args.tag,
        page_cursor: args.cursor,
        with_html_content: args.with_html_content,
        with_raw_source_url: args.with_raw_source_url,
    };

    let mut page_num = 1;

    loop {
        // Generate cache key for this page
        let cache_key = format!(
            "list:{}:{}:{}:{}:page:{}",
            params.location.as_deref().unwrap_or("all"),
            params.category.as_deref().unwrap_or("all"),
            params.tag.as_deref().unwrap_or("all"),
            params.id.as_deref().unwrap_or("all"),
            page_num
        );

        // Try to get from cache first
        let response = if let Some(c) = cache.as_ref() {
            if let Some(entry) = c.get(&cache_key) {
                // Cache hit - deserialize the response
                serde_json::from_value::<ListDocumentsResponse>(entry.response.clone()).ok()
            } else {
                None
            }
        } else {
            None
        };

        // If not in cache, fetch from API
        let response = if let Some(cached_response) = response {
            cached_response
        } else {
            let api_response = client.list_documents(&params).await?;

            // Store in cache
            if let Some(c) = cache.as_mut() {
                let params_json = serde_json::json!({
                    "location": params.location,
                    "category": params.category,
                    "tag": params.tag,
                    "id": params.id,
                    "page": page_num
                });
                let response_json = serde_json::to_value(&api_response)?;
                c.set(&cache_key, "list", params_json, response_json);
            }

            api_response
        };

        // Print page results
        if json_output {
            println!("{}", serde_json::to_string(&response).unwrap_or_default());
        } else {
            println!(
                "=== Page {} (showing {}/{} total) ===",
                page_num,
                response.results.len(),
                response.count
            );
            println!("{}", output::format_list_response(&response, false));
        }

        // Check if there's a next page
        match response.next_page_cursor {
            Some(cursor) => {
                if args.all {
                    // Auto-fetch next page
                    params.page_cursor = Some(cursor);
                    page_num += 1;
                } else {
                    // Wait for user input
                    eprint!("Press Enter for next page (or 'q' to quit): ");
                    io::stderr().flush().ok();

                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_err() {
                        break;
                    }

                    let input = input.trim().to_lowercase();
                    if input == "q" || input == "quit" {
                        break;
                    }

                    params.page_cursor = Some(cursor);
                    page_num += 1;
                }
            }
            None => {
                // No more pages
                if !json_output {
                    eprintln!("--- End of results ---");
                }
                break;
            }
        }
    }

    Ok(())
}

async fn handle_update(
    client: &mut ReaderClient,
    args: UpdateArgs,
    json_output: bool,
) -> Result<()> {
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

async fn handle_delete(client: &mut ReaderClient, id: &str, json_output: bool) -> Result<()> {
    client.delete_document(id).await?;
    println!("{}", output::format_delete_response(id, json_output));
    Ok(())
}

async fn handle_tag_list(
    client: &mut ReaderClient,
    json_output: bool,
    cache: &mut Option<Cache>,
) -> Result<()> {
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
