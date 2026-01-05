use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

#[derive(Parser, Debug)]
#[command(name = "rr")]
#[command(version, about = "Readwise Reader API CLI", long_about = None)]
#[command(arg_required_else_help = true)]
pub struct Args {
    /// Readwise API access token
    #[arg(long, env = "READWISE_ACCESS_TOKEN", global = true)]
    pub token: Option<String>,

    /// Enable caching of API responses
    #[arg(long, global = true, default_value_t = true)]
    pub cache: bool,

    /// Cache file path
    #[arg(long, global = true, default_value = "./rr_cache.json")]
    pub cache_file: String,

    /// Output raw JSON instead of pretty format
    #[arg(long, global = true, default_value_t = false)]
    pub json: bool,

    /// Enable verbose debug output (prints HTTP requests/responses)
    #[arg(short, long, global = true, default_value_t = false)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Check API authentication status
    Auth,

    /// Create a new document
    Create(CreateArgs),

    /// List documents
    List(ListArgs),

    /// Update an existing document
    Update(UpdateArgs),

    /// Delete a document
    Delete(DeleteArgs),

    /// List all tags
    TagList,
}

#[derive(clap::Args, Debug)]
pub struct CreateArgs {
    /// URL of the document (required)
    #[arg(long)]
    pub url: String,

    /// HTML content of the document
    #[arg(long)]
    pub html: Option<String>,

    /// Whether to clean the HTML content
    #[arg(long)]
    pub should_clean_html: Option<bool>,

    /// Document title
    #[arg(long)]
    pub title: Option<String>,

    /// Document author
    #[arg(long)]
    pub author: Option<String>,

    /// Document summary
    #[arg(long)]
    pub summary: Option<String>,

    /// Published date (ISO 8601 format)
    #[arg(long)]
    pub published_date: Option<String>,

    /// Image URL for the document
    #[arg(long)]
    pub image_url: Option<String>,

    /// Location: new, later, archive, feed
    #[arg(long, value_enum)]
    pub location: Option<Location>,

    /// Category: article, email, rss, highlight, note, pdf, epub, tweet, video
    #[arg(long, value_enum)]
    pub category: Option<Category>,

    /// Application that saved the document
    #[arg(long)]
    pub saved_using: Option<String>,

    /// Tags (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,

    /// Notes for the document
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct ListArgs {
    /// Fetch specific document by ID
    #[arg(long)]
    pub id: Option<String>,

    /// Filter documents updated after this date (ISO 8601)
    #[arg(long)]
    pub updated_after: Option<String>,

    /// Filter by location
    #[arg(long, value_enum)]
    pub location: Option<ListLocation>,

    /// Filter by category
    #[arg(long, value_enum)]
    pub category: Option<Category>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Pagination cursor
    #[arg(long)]
    pub cursor: Option<String>,

    /// Include HTML content in response
    #[arg(long)]
    pub with_html_content: Option<bool>,

    /// Include raw source URL in response
    #[arg(long)]
    pub with_raw_source_url: Option<bool>,

    /// Fetch all pages without waiting for user input
    #[arg(long, short)]
    pub all: bool,
}

#[derive(clap::Args, Debug)]
pub struct UpdateArgs {
    /// Document ID to update (required)
    pub id: String,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New author
    #[arg(long)]
    pub author: Option<String>,

    /// New summary
    #[arg(long)]
    pub summary: Option<String>,

    /// New published date (ISO 8601)
    #[arg(long)]
    pub published_date: Option<String>,

    /// New image URL
    #[arg(long)]
    pub image_url: Option<String>,

    /// Mark document as seen
    #[arg(long)]
    pub seen: Option<bool>,

    /// New location
    #[arg(long, value_enum)]
    pub location: Option<Location>,

    /// New category
    #[arg(long, value_enum)]
    pub category: Option<Category>,

    /// New tags (comma-separated, replaces existing)
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,
}

#[derive(clap::Args, Debug)]
pub struct DeleteArgs {
    /// Document ID to delete
    pub id: String,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Location {
    New,
    Later,
    Archive,
    Feed,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::New => write!(f, "new"),
            Location::Later => write!(f, "later"),
            Location::Archive => write!(f, "archive"),
            Location::Feed => write!(f, "feed"),
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ListLocation {
    New,
    Later,
    Shortlist,
    Archive,
    Feed,
}

impl fmt::Display for ListLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListLocation::New => write!(f, "new"),
            ListLocation::Later => write!(f, "later"),
            ListLocation::Shortlist => write!(f, "shortlist"),
            ListLocation::Archive => write!(f, "archive"),
            ListLocation::Feed => write!(f, "feed"),
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Category {
    Article,
    Email,
    Rss,
    Highlight,
    Note,
    Pdf,
    Epub,
    Tweet,
    Video,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Article => write!(f, "article"),
            Category::Email => write!(f, "email"),
            Category::Rss => write!(f, "rss"),
            Category::Highlight => write!(f, "highlight"),
            Category::Note => write!(f, "note"),
            Category::Pdf => write!(f, "pdf"),
            Category::Epub => write!(f, "epub"),
            Category::Tweet => write!(f, "tweet"),
            Category::Video => write!(f, "video"),
        }
    }
}
