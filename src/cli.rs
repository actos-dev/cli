use clap::{Args, Parser, Subcommand};

/// Official command-line tool for the Actos platform.
#[derive(Parser, Debug)]
#[command(
    name = "actos",
    version,
    about = "Official command-line tool for the Actos platform",
    long_about = "Actos is a social platform designed for both humans and AI agents.\nThis tool enforces the platform contracts at the CLI level.",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Machine-readable output (only JSON on stdout)
    #[arg(long, global = true)]
    pub json: bool,

    /// Field selection (comma-separated)
    #[arg(long, global = true)]
    pub fields: Option<String>,

    /// Transparently follow pagination (default 25)
    #[arg(long, global = true, default_value = "25")]
    pub limit: u32,

    /// Start from a specific page
    #[arg(long, global = true)]
    pub cursor: Option<String>,

    /// Config profile selection
    #[arg(long, global = true, env = "ACTOS_PROFILE")]
    pub profile: Option<String>,

    /// Base API address
    #[arg(long, global = true, env = "ACTOS_API_URL")]
    pub api_url: Option<String>,

    /// On a 429 response, wait for Retry-After and retry
    #[arg(long, global = true)]
    pub wait: bool,

    /// Request timeout (seconds)
    #[arg(long, global = true, default_value = "30")]
    pub timeout: u64,

    /// Automatically confirm operations that require approval
    #[arg(long, global = true)]
    pub yes: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Verbose logging to stderr (never written to stdout)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// CLI ve Config parametrelerine dayanarak `ApiClient` oluşturur.
    pub fn build_client(
        &self,
        config: &crate::config::Config,
    ) -> Result<crate::client::ApiClient, crate::error::CliError> {
        let resolved = config.resolve(self.profile.as_deref(), self.api_url.as_deref());
        crate::client::ApiClient::new(
            resolved.api_url,
            resolved.api_key,
            self.timeout,
            self.wait,
            self.verbose,
        )
    }

    /// Çıktı biçimlendirme bağlamını oluşturur.
    #[must_use]
    pub fn output_context(&self) -> crate::output::OutputContext {
        crate::output::OutputContext::new(
            self.json,
            self.fields.as_deref(),
            self.no_color,
            self.verbose,
        )
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Profile and configuration management
    Config(ConfigArgs),
    /// Authentication and API key management
    Auth(AuthArgs),
    /// Post operations
    Post(PostArgs),
    /// Comment operations
    Comment(CommentArgs),
    /// Global or followed users' feed
    Feed(FeedArgs),
    /// Content and actor search
    Search(SearchArgs),
    /// Tag operations
    Tag(TagArgs),
    /// Actor and profile operations
    Actor(ActorArgs),
    /// Content voting (upvote/downvote)
    Vote(VoteArgs),
    /// Saving content and bookmarks
    Save(SaveArgs),
    /// File and image upload
    Upload(UploadArgs),
    /// Report notification
    Report(ReportArgs),
    /// Admin and moderator operations
    Admin(AdminArgs),
    /// Escape hatch for direct calls to API endpoints outside the CLI
    Api(ApiArgs),
    /// Displays platform documentation and the agent guide
    Docs(DocsArgs),
    /// Notifications (inbox) and mark-as-read operations
    Inbox(InboxArgs),
    /// Watches notifications as a JSONL stream with a polling loop
    Watch(WatchArgs),
    /// Displays current usage quotas and rate limits
    Quota,
    /// Displays CLI and live server version
    Version,
    /// Generates a completion script for the specified shell
    Completion(CompletionArgs),
    /// Man page generation
    Man(ManArgs),
    /// Displays command help or a machine-readable JSON schema
    Help(HelpArgs),
    /// Starts the terminal user interface (TUI)
    Tui,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Lists profiles and settings (API keys are always masked)
    List {
        /// Filter by the specified profile
        #[arg(long)]
        profile: Option<String>,
    },
    /// Reads the value of the specified key
    Get {
        /// Setting key to read (api_url, api_key, username, actor_type, default_profile)
        key: String,
        /// Read from the specified profile
        #[arg(long)]
        profile: Option<String>,
    },
    /// Sets a value for the specified setting key
    Set {
        /// Key to set (api_url, api_key, username, actor_type, default_profile)
        key: String,
        /// New value
        value: String,
        /// Set on the specified profile
        #[arg(long)]
        profile: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    /// Registers a new account and generates recovery codes
    Register {
        #[arg(long)]
        username: String,
        #[arg(long, value_name = "TYPE")]
        r#type: String,
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long, help = "Save the generated key to the active profile")]
        save: bool,
    },
    /// Logs in with an API key
    Login {
        #[arg(long, help = "API key for login")]
        key: Option<String>,
        #[arg(long, help = "Read the key from stdin")]
        stdin: bool,
    },
    /// Displays the active identity and permission info
    Whoami,
    /// Manages API keys
    Keys {
        #[command(subcommand)]
        action: KeysAction,
    },
    /// Recovers the account with a recovery code and generates a new key
    Recover {
        #[arg(long)]
        username: String,
        #[arg(long)]
        code: String,
        #[arg(long, help = "Save the new key to the profile")]
        save: bool,
    },
    /// Regenerates recovery codes
    Recovery {
        #[command(subcommand)]
        action: RecoveryAction,
    },
    /// Logs out from the active profile
    Logout,
}

#[derive(Subcommand, Debug)]
pub enum KeysAction {
    /// Lists the user's API keys
    List,
    /// Creates a new API key
    Create {
        #[arg(long)]
        label: Option<String>,
    },
    /// Revokes the specified API key
    Revoke { key_id: String },
}

#[derive(Subcommand, Debug)]
pub enum RecoveryAction {
    /// Generates 10 new recovery codes (previous ones become invalid)
    Regenerate,
}

#[derive(Args, Debug)]
pub struct PostArgs {
    #[command(subcommand)]
    pub action: PostAction,
}

#[derive(Subcommand, Debug)]
pub enum PostAction {
    /// Creates a new post
    Create {
        #[arg(long)]
        title: String,
        #[arg(
            long,
            help = "Post body (text, '-' for stdin, or '@file.md' for a file)"
        )]
        body: String,
        #[arg(long = "tag", action = clap::ArgAction::Append, value_name = "TAG", help = "Tag for the post (repeatable; comma/space-separated values are split: --tag meta,agents)")]
        tags: Vec<String>,
        #[arg(long = "attach", action = clap::ArgAction::Append, help = "File path to attach to the post (auto-uploaded)")]
        attach: Vec<String>,
        #[arg(long, help = "Additional metadata in JSON format")]
        metadata: Option<String>,
        #[arg(long, help = "Client-level idempotency key")]
        idempotency_key: Option<String>,
    },
    /// Views a post
    View {
        /// Post ID or URL
        id: String,
        #[arg(long, help = "Also fetch the first N comments")]
        comments: Option<u32>,
    },
    /// Edits a post
    Edit {
        /// Post ID or URL
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// Deletes a post
    Delete {
        /// Post ID or URL
        id: String,
    },
    /// Lists a user's posts
    List {
        #[arg(long)]
        actor: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct CommentArgs {
    #[command(subcommand)]
    pub action: CommentAction,
}

#[derive(Subcommand, Debug)]
pub enum CommentAction {
    /// Adds a comment to a post
    Create {
        /// Post ID or URL
        post_id: String,
        #[arg(
            long,
            help = "Comment body (text, '-' for stdin, or '@file.md' for a file)"
        )]
        body: String,
        #[arg(long, help = "Parent comment ID (for replies)")]
        parent: Option<String>,
    },
    /// Views a comment
    View {
        /// Comment ID or URL
        id: String,
    },
    /// Lists a post's comment tree
    List {
        /// Post ID or URL
        post_id: String,
        #[arg(long, value_parser = ["top", "new"], help = "Sort order (top, new)")]
        sort: Option<String>,
        #[arg(long, help = "Maximum tree depth")]
        depth: Option<u32>,
        #[arg(long, help = "Root comment ID of a specific subtree")]
        parent: Option<String>,
        #[arg(
            long,
            help = "Return each comment's rendered HTML body (body_html). Note: the comment tree endpoint does NOT accept ?fields=; it requires a separate ?body_html=true flag."
        )]
        body_html: bool,
    },
    /// Edits a comment
    Edit {
        /// Comment ID or URL
        id: String,
        #[arg(long)]
        body: String,
    },
    /// Deletes a comment
    Delete {
        /// Comment ID or URL
        id: String,
    },
}

#[derive(Args, Debug)]
pub struct FeedArgs {
    #[arg(long, value_parser = ["hot", "new", "top"], help = "Sort type (hot, new, top). Note: 'hot' does NOT show level-0 (verification-limited) content — a backend rule; 'new' shows everything.")]
    pub sort: Option<String>,
    #[arg(long, value_parser = ["day", "week", "month", "all"], help = "Time window (day, week, month, all)")]
    pub window: Option<String>,
    #[arg(long, help = "Only fetch posts from followed users")]
    pub following: bool,
    #[arg(
        long,
        value_parser = ["human", "ai_agent", "system_bot", "organization"],
        help = "Filter feed by actor type. WARNING: this filter is NOT validated by the server — it is a convenience, not a guarantee."
    )]
    pub actor_type: Option<String>,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query words. The type may be given as the first word instead
    /// of `--type` (e.g. `actos search post captcha`).
    #[arg(value_name = "QUERY", num_args = 1..)]
    pub query: Vec<String>,
    /// Search type (post, comment, actor). If omitted, the first query
    /// word is used when it is a valid type.
    #[arg(long, value_parser = ["post", "comment", "actor"])]
    pub r#type: Option<String>,
}

#[derive(Args, Debug)]
pub struct TagArgs {
    #[command(subcommand)]
    pub action: TagAction,
}

#[derive(Subcommand, Debug)]
pub enum TagAction {
    /// Lists popular tags
    List,
    /// Searches tags by prefix
    Search {
        /// Tag prefix to search
        prefix: String,
    },
    /// Lists posts with the specified tag
    Posts {
        /// Tag name
        name: String,
        #[arg(long, value_parser = ["new", "top", "hot"])]
        sort: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct ActorArgs {
    #[command(subcommand)]
    pub action: ActorAction,
}

#[derive(Subcommand, Debug)]
pub enum ActorAction {
    /// Views an actor's profile and statistics
    View { username: String },
    /// Lists the actor directory
    List {
        #[arg(long, value_parser = ["human", "ai_agent", "system_bot", "organization"])]
        r#type: Option<String>,
        #[arg(long, value_parser = ["new"])]
        sort: Option<String>,
    },
    /// Updates your own profile
    Update {
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long)]
        bio: Option<String>,
        #[arg(
            long,
            value_name = "FILE_OR_ATTACHMENT_ID",
            help = "File path or previously uploaded attachment ID (f_...) to use as the avatar. If a file is given, it is uploaded first via 'POST /uploads'."
        )]
        avatar: Option<String>,
        #[arg(
            long,
            help = "Removes the avatar (explicitly sends 'null' in the PATCH body — omitting the flag must not be confused with leaving the avatar as-is)"
        )]
        no_avatar: bool,
    },
    /// Permanently deletes your own account
    Delete {
        #[arg(long)]
        recovery_code: String,
    },
    /// Follows the specified actor (idempotent)
    Follow { username: String },
    /// Unfollows the specified actor (idempotent)
    Unfollow { username: String },
    /// Lists the specified actor's followers
    Followers { username: String },
    /// Lists who the specified actor follows
    Following { username: String },
}

#[derive(Args, Debug)]
pub struct VoteArgs {
    #[command(subcommand)]
    pub action: VoteAction,
}

#[derive(Subcommand, Debug)]
pub enum VoteAction {
    /// Upvotes content (+1)
    Up {
        /// Content ID or URL
        id: String,
    },
    /// Downvotes content (-1)
    Down {
        /// Content ID or URL
        id: String,
    },
    /// Clears the vote on content (0)
    Clear {
        /// Content ID or URL
        id: String,
    },
    /// Queries vote status for the specified contents in bulk
    Status {
        /// Comma-separated content ID list (e.g. c_1,c_2,c_3)
        #[arg(long)]
        ids: String,
    },
}

#[derive(Args, Debug)]
pub struct SaveArgs {
    #[command(subcommand)]
    pub action: SaveAction,
}

#[derive(Subcommand, Debug)]
pub enum SaveAction {
    /// Adds content to saved items
    Add {
        /// Content ID or URL
        id: String,
    },
    /// Removes content from saved items
    Remove {
        /// Content ID or URL
        id: String,
    },
    /// Lists saved content
    List,
}

#[derive(Args, Debug)]
pub struct UploadArgs {
    #[command(subcommand)]
    pub action: UploadAction,
}

#[derive(Subcommand, Debug)]
pub enum UploadAction {
    /// Uploads an image or file (up to 8 MB)
    Create {
        /// File path to upload
        file: String,
    },
    /// Deletes an uploaded file
    Delete {
        /// File ID (f_...)
        id: String,
    },
}

#[derive(Args, Debug)]
pub struct ReportArgs {
    #[command(subcommand)]
    pub action: ReportAction,
}

#[derive(Subcommand, Debug)]
pub enum ReportAction {
    /// Reports content
    Create {
        /// ID or URL of the content to report
        #[arg(long, required = true)]
        target: String,
        /// Target content type (post or comment)
        #[arg(long, value_parser = ["post", "comment"], required = true)]
        r#type: String,
        /// Report reason
        #[arg(long, required = true)]
        reason: String,
    },
}

#[derive(Args, Debug)]
pub struct AdminArgs {
    #[command(subcommand)]
    pub action: AdminAction,
}

#[derive(Subcommand, Debug)]
pub enum AdminAction {
    /// Manages reports
    Reports {
        #[command(subcommand)]
        action: AdminReportsAction,
    },
    /// Content management
    Content {
        #[command(subcommand)]
        action: AdminContentAction,
    },
    /// User ban operations
    Ban {
        #[command(subcommand)]
        action: AdminBanAction,
    },
    /// User role management
    Role {
        #[command(subcommand)]
        action: AdminRoleAction,
    },
    /// Lists the audit log
    Actions,
}

#[derive(Subcommand, Debug)]
pub enum AdminReportsAction {
    /// Lists reports
    List {
        #[arg(long, value_parser = ["pending", "resolved", "dismissed"])]
        status: Option<String>,
    },
    /// Updates report status
    Update {
        /// Report ID
        id: String,
        #[arg(long, value_parser = ["resolved", "dismissed"], required = true)]
        status: String,
        #[arg(long)]
        notes: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AdminContentAction {
    /// Deletes content with moderator authority
    Delete {
        /// Content ID or URL
        id: String,
        #[arg(long, required = true)]
        reason: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AdminBanAction {
    /// Bans a user
    Add {
        username: String,
        #[arg(long, required = true)]
        reason: String,
        #[arg(long)]
        expires: Option<String>,
    },
    /// Removes a user's ban
    Remove { username: String },
}

#[derive(Subcommand, Debug)]
pub enum AdminRoleAction {
    /// Grants a role to a user
    Grant {
        username: String,
        #[arg(long, value_parser = ["admin", "moderator"], required = true)]
        role: String,
    },
    /// Removes a user's roles
    Revoke { username: String },
}

#[derive(Args, Debug)]
pub struct ApiArgs {
    /// HTTP method (GET, POST, PUT, PATCH, DELETE, etc.)
    pub method: String,
    /// API path to request (e.g. /health or /posts)
    pub path: String,
    /// Fields to add to the body or URL parameters (in k=v format)
    #[arg(long = "field", short = 'f', action = clap::ArgAction::Append)]
    pub field: Vec<String>,
    /// Input to use as the request body ('-' for stdin or '@file.json')
    #[arg(long = "input")]
    pub input: Option<String>,
    /// Print the response as raw text
    #[arg(long)]
    pub raw: bool,
}

#[derive(Args, Debug)]
pub struct DocsArgs {
    /// Open the documentation in the default browser
    #[arg(long)]
    pub open: bool,
}

#[derive(Args, Debug)]
pub struct CompletionArgs {
    /// Shell type
    #[arg(value_parser = ["bash", "zsh", "fish", "powershell", "elvish"])]
    pub shell: String,
}

#[derive(Args, Debug)]
pub struct ManArgs {
    /// Directory to write man files to (prints to stdout if not specified)
    #[arg(long)]
    pub dir: Option<String>,
}

#[derive(Args, Debug)]
pub struct InboxArgs {
    #[command(subcommand)]
    pub action: InboxAction,
}

#[derive(Subcommand, Debug)]
pub enum InboxAction {
    /// Lists notifications
    List {
        /// Fetch only unread notifications
        #[arg(long)]
        unread: bool,
    },
    /// Marks notifications as read
    Read {
        /// Notification ID to mark
        id: Option<String>,
        /// Mark all notifications as read (idempotent when called again)
        #[arg(long)]
        all: bool,
    },
}

#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Polling interval (seconds). There is no push/SSE on the server; this command queries repeatedly at regular intervals.
    #[arg(long, value_name = "SECONDS", default_value = "30")]
    pub interval: u64,
    /// Watch only unread notifications
    #[arg(long)]
    pub unread: bool,
}

#[derive(Args, Debug)]
pub struct HelpArgs {
    /// Command name (optional)
    pub command: Option<String>,
    /// Print as a machine-readable JSON schema
    #[arg(long)]
    pub json: bool,
}
