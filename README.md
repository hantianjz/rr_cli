# rr - Readwise Reader CLI

A command-line tool for interacting with the [Readwise Reader](https://readwise.io/read) API.

## Installation

```bash
cargo build --release
# Binary is at ./target/release/rr
```

## Authentication

Set your Readwise access token (get it from https://readwise.io/access_token):

```bash
export READWISE_ACCESS_TOKEN="your_token_here"
```

Or pass it directly with `--token`.

## Commands

```bash
rr auth              # Verify your token is valid
rr create --url URL  # Save a new document
rr list              # List your documents
rr update ID         # Update a document
rr delete ID         # Delete a document
rr tag-list          # List all your tags
```

## Examples

```bash
# Save an article
rr create --url "https://example.com/article" --tags "reading,tech"

# List documents in your "later" list
rr list --location later

# List articles only
rr list --category article

# Move a document to archive
rr update abc123 --location archive

# Get raw JSON output
rr --json list
```

## Features

- **Caching**: API responses are cached locally to `rr_cache.json`
- **Debug mode**: Use `-v` to see HTTP requests/responses and save them to `debug_cache.json`
- **Flexible output**: Pretty output by default, `--json` for raw JSON

## Options

| Option | Description |
|--------|-------------|
| `--token` | API token (or set `READWISE_ACCESS_TOKEN`) |
| `--cache-file` | Cache file path (default: `./rr_cache.json`) |
| `--json` | Output raw JSON |
| `-v, --verbose` | Debug mode |

## API Reference

See https://readwise.io/reader_api for full API documentation.
