use crate::client::ApiClient;
use actos_types::actor::ActorProfileResponse;
use actos_types::content::{CommentNodeResponse, ContentSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentTab {
    Feed,
    Detail,
    Search,
    Profile,
    Help,
}

pub struct App {
    pub current_tab: CurrentTab,
    pub previous_tab: CurrentTab,
    pub feed_posts: Vec<ContentSummary>,
    pub feed_selected: usize,
    pub selected_post: Option<ContentSummary>,
    pub post_comments: Vec<CommentNodeResponse>,
    pub search_query: String,
    pub search_results: Vec<ContentSummary>,
    pub search_selected: usize,
    pub profile_info: Option<ActorProfileResponse>,
    pub status_message: String,
    pub show_help_popup: bool,
    pub should_quit: bool,
}

impl App {
    pub async fn new(client: &ApiClient) -> Self {
        let mut app = Self {
            current_tab: CurrentTab::Feed,
            previous_tab: CurrentTab::Feed,
            feed_posts: Vec::new(),
            feed_selected: 0,
            selected_post: None,
            post_comments: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            profile_info: None,
            status_message: "Ready".to_string(),
            show_help_popup: false,
            should_quit: false,
        };

        app.refresh_feed(client).await;
        app.refresh_profile(client).await;
        app
    }

    pub async fn refresh_feed(&mut self, client: &ApiClient) {
        self.status_message = "Loading feed...".to_string();
        match client.paginate("/feed", &[], 25, None).await {
            Ok((val, _rl)) => {
                if let Some(posts_val) = val.get("posts")
                    && let Ok(posts) =
                        serde_json::from_value::<Vec<ContentSummary>>(posts_val.clone())
                {
                    self.feed_posts = posts;
                    self.feed_selected = 0;
                    self.status_message = format!("Feed loaded ({} posts)", self.feed_posts.len());
                    return;
                }
                self.status_message = "Feed loaded (0 posts)".to_string();
            }
            Err(e) => {
                self.status_message = format!("Error loading feed: {e}");
            }
        }
    }

    pub async fn open_selected_post(&mut self, client: &ApiClient) {
        if self.feed_posts.is_empty() || self.feed_selected >= self.feed_posts.len() {
            return;
        }

        let post = self.feed_posts[self.feed_selected].clone();
        let post_id = post.id.clone();
        self.selected_post = Some(post);
        self.previous_tab = self.current_tab;
        self.current_tab = CurrentTab::Detail;

        self.status_message = format!("Loading comments for {post_id}...");
        let comments_path = format!("/posts/{post_id}/comments");
        match client.get_json(&comments_path, None).await {
            Ok((val, _rl)) => {
                if let Some(thread_val) = val.get("thread")
                    && let Ok(comments) =
                        serde_json::from_value::<Vec<CommentNodeResponse>>(thread_val.clone())
                {
                    self.post_comments = comments;
                    self.status_message = "Comments loaded".to_string();
                    return;
                }
                self.post_comments.clear();
                self.status_message = "No comments".to_string();
            }
            Err(e) => {
                self.post_comments.clear();
                self.status_message = format!("Could not load comments: {e}");
            }
        }
    }

    pub async fn perform_search(&mut self, client: &ApiClient) {
        if self.search_query.trim().is_empty() {
            return;
        }

        self.status_message = format!("Searching for '{}'...", self.search_query);
        let query = [("q", self.search_query.as_str()), ("type", "post")];
        match client.get_json("/search", Some(&query)).await {
            Ok((val, _rl)) => {
                if let Some(results_val) = val.get("results")
                    && let Ok(results) =
                        serde_json::from_value::<Vec<ContentSummary>>(results_val.clone())
                {
                    self.search_results = results;
                    self.search_selected = 0;
                    self.status_message = format!("Found {} results", self.search_results.len());
                    return;
                }
                self.search_results.clear();
                self.status_message = "No results found".to_string();
            }
            Err(e) => {
                self.search_results.clear();
                self.status_message = format!("Search failed: {e}");
            }
        }
    }

    pub async fn refresh_profile(&mut self, client: &ApiClient) {
        if client.api_key().is_none() {
            self.profile_info = None;
            return;
        }

        if let Ok((whoami, _rl)) = client.get_json("/auth/whoami", None).await
            && let Some(username) = whoami["actor"]["username"].as_str()
        {
            let profile_path = format!("/actors/{username}");
            if let Ok((profile_val, _rl)) = client.get_json(&profile_path, None).await
                && let Ok(profile) = serde_json::from_value::<ActorProfileResponse>(profile_val)
            {
                self.profile_info = Some(profile);
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            CurrentTab::Feed => CurrentTab::Search,
            CurrentTab::Search => CurrentTab::Profile,
            CurrentTab::Profile => CurrentTab::Help,
            CurrentTab::Help => CurrentTab::Feed,
            CurrentTab::Detail => CurrentTab::Feed,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            CurrentTab::Feed => CurrentTab::Help,
            CurrentTab::Search => CurrentTab::Feed,
            CurrentTab::Profile => CurrentTab::Search,
            CurrentTab::Help => CurrentTab::Profile,
            CurrentTab::Detail => CurrentTab::Feed,
        };
    }

    pub fn move_up(&mut self) {
        match self.current_tab {
            CurrentTab::Feed if self.feed_selected > 0 => {
                self.feed_selected -= 1;
            }
            CurrentTab::Search if self.search_selected > 0 => {
                self.search_selected -= 1;
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.current_tab {
            CurrentTab::Feed
                if !self.feed_posts.is_empty()
                    && self.feed_selected + 1 < self.feed_posts.len() =>
            {
                self.feed_selected += 1;
            }
            CurrentTab::Search
                if !self.search_results.is_empty()
                    && self.search_selected + 1 < self.search_results.len() =>
            {
                self.search_selected += 1;
            }
            _ => {}
        }
    }
}
