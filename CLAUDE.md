# rr_cli - Readwise Reader CLI

A Rust CLI tool for the Readwise Reader API.

## Build & Run

```bash
cargo build --release
./target/release/rr --help
```

## Authentication

Set the `READWISE_ACCESS_TOKEN` environment variable or use `--token`:

```bash
export READWISE_ACCESS_TOKEN="your_token"
rr auth
# or
rr --token "your_token" auth
```

## Commands

| Command | Description |
|---------|-------------|
| `rr auth` | Check API authentication |
| `rr create --url <URL>` | Create a document |
| `rr list` | List documents |
| `rr update <ID>` | Update a document |
| `rr delete <ID>` | Delete a document |
| `rr tag_list` | List all tags |

## Global Options

- `--token <TOKEN>` - API token (env: `READWISE_ACCESS_TOKEN`)
- `--cache` - Enable response caching
- `--cache-file <PATH>` - Cache file path (default: `./rr_cache.json`)
- `--json` - Output raw JSON instead of pretty format

## Examples

```bash
# Create a document
rr create --url "https://example.com/article" --title "My Article" --tags "rust,programming"

# List documents in "new" location
rr list --location new

# Update a document
rr update abc123 --location archive --seen true

# Delete a document
rr delete abc123

# List all tags
rr tag_list

# With caching enabled
rr --cache list --location new
```

## Project Structure

```
src/
├── main.rs      # Entry point, command dispatch
├── cli.rs       # Clap argument definitions
├── client.rs    # HTTP client with auth
├── types.rs     # Request/response structs
├── cache.rs     # JSON file caching
├── output.rs    # Output formatting
└── error.rs     # Custom error types
```

## Rate Limiting

- Internal limit: 20 API requests per session (`MAX_REQUESTS_PER_SESSION`)
- Readwise API limit: 20 requests/minute (50 for create/update)
- `tag_list` fetches all pages automatically within the internal limit

## API Reference

See https://readwise.io/reader_api for full API documentation.
