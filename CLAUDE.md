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
| `rr tag-list` | List all tags |

## Global Options

- `--token <TOKEN>` - API token (env: `READWISE_ACCESS_TOKEN`)
- `--cache` - Enable response caching
- `--cache-file <PATH>` - Cache file path (default: `./rr_cache.json`)
- `--json` - Output raw JSON instead of pretty format
- `-v, --verbose` - Enable debug output (prints HTTP requests/responses)

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
rr tag-list

# With caching enabled
rr --cache list --location new

# Debug mode (prints HTTP requests/responses, saves to debug_cache.json)
rr -v auth
rr --verbose list --location new
```

## Project Structure

```
src/
├── main.rs      # Entry point, command dispatch
├── cli.rs       # Clap argument definitions
├── client.rs    # HTTP client with auth and retry logic
├── types.rs     # Request/response structs
├── cache.rs     # JSON file caching
└── output.rs    # Output formatting
```

## Rate Limiting

The tool handles rate limiting dynamically based on API responses:

- **No internal rate limiting**: No artificial request limits - the tool makes as many requests as needed
- **Automatic retry on 429**: When the Readwise API returns a 429 (Too Many Requests) error:
  1. Parses the wait time from the error message (e.g., "Expected available in 9 seconds")
  2. Displays a countdown timer on the same line showing remaining wait time
  3. Automatically retries the request after the wait period completes
- **Readwise API limits**: 20 requests/minute for most operations (50 for create/update)
- **Pagination**: Commands like `tag-list` and `list --all` fetch all pages automatically, handling rate limits transparently

## Debug Cache

When `--verbose` mode is enabled, all HTTP requests and responses are saved to `debug_cache.json`:

```json
{
  "entries": [
    {
      "timestamp": "2024-01-04T12:00:00Z",
      "method": "GET",
      "url": "https://readwise.io/api/v3/list/",
      "request_body": null,
      "status": 200,
      "response_body": { ... }
    }
  ]
}
```

## API Reference

See https://readwise.io/reader_api for full API documentation.
