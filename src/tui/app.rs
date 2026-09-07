use crate::client::ApiClient;
use crate::tui::mouse::MouseAction;
use actos_sdk::actos_types::actor::ActorProfileResponse;
use actos_sdk::actos_types::content::{CommentNodeResponse, ContentSummary};
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentTab {
    Feed,
    Detail,
    Search,
    Profile,
    Help,
}

/// Başlıkla aynı olan ilk gövde başlığını düşürür (web-lite kuralı).
///
/// Yazarlar başlığı gövdeye de `# Başlık` diye yazıyor; aynen basılınca
/// başlık iki kere görünüyor. Yalnızca görüntü katmanı, veri değişmez.
#[must_use]
pub fn strip_leading_title(body: &str, title: Option<&str>) -> String {
    fn norm(s: &str) -> String {
        s.chars()
            .filter(|c| !matches!(c, '*' | '_' | '`' | '~'))
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }
    let Some(t) = title else {
        return body.to_string();
    };
    if norm(t).is_empty() {
        return body.to_string();
    }
    let mut lines: Vec<&str> = body.lines().collect();
    let first = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(lines.len());
    if first < lines.len() {
        let trimmed = lines[first].trim();
        let marks = trimmed.chars().take_while(|c| *c == '#').count();
        // `#` (1-6 adet) + boşluk + metin: klasik markdown başlığı.
        // `#` ve boşluk tek bayt olduğundan bayt dilimi güvenlidir.
        if (1..=6).contains(&marks) && trimmed.chars().nth(marks) == Some(' ') {
            let heading = trimmed[marks + 1..].trim();
            if norm(heading) == norm(t) {
                lines.remove(first);
                return lines.join("\n");
            }
        }
    }
    body.to_string()
}

pub struct App {
    pub current_tab: CurrentTab,
    /// Geri yığını: Detail gibi üst ekranlara buradan dönülür (Esc).
    pub back_stack: Vec<CurrentTab>,
    pub feed: PagedList<ContentSummary>,
    pub selected_post: Option<ContentSummary>,
    pub post_comments: Vec<CommentNodeResponse>,
    pub search_query: String,
    pub search_type: String,
    pub search: PagedList<ContentSummary>,
    pub feed_sort: String,
    pub feed_window: String,
    pub feed_actor_type: Option<String>,
    pub feed_following: bool,
    pub profile_info: Option<ActorProfileResponse>,
    pub status_message: String,
    pub show_help_popup: bool,
    pub should_quit: bool,
    /// O karede tıklanabilir bölgeler (her çizimde baştan doldurulur).
    pub hit_areas: Vec<(Rect, MouseAction)>,
}

/// Sayfalanabilir seçili liste: tüm liste ekranlarının ortak altyapısı.
#[derive(Debug)]
pub struct PagedList<T> {
    pub items: Vec<T>,
    pub selected: usize,
    pub cursor: Option<String>,
}

impl<T> Default for PagedList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            cursor: None,
        }
    }
}

impl<T> PagedList<T> {
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.selected = 0;
        self.cursor = None;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.items.is_empty() && self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    #[must_use]
    pub fn selected_item(&self) -> Option<&T> {
        self.items.get(self.selected)
    }
}

impl App {
    pub async fn new(client: &ApiClient) -> Self {
        let mut app = Self {
            current_tab: CurrentTab::Feed,
            back_stack: Vec::new(),
            feed: PagedList::default(),
            selected_post: None,
            post_comments: Vec::new(),
            search_query: String::new(),
            search_type: "post".to_string(),
            search: PagedList::default(),
            feed_sort: "new".to_string(),
            feed_window: "all".to_string(),
            feed_actor_type: None,
            feed_following: false,
            profile_info: None,
            status_message: "Ready".to_string(),
            show_help_popup: false,
            should_quit: false,
            hit_areas: Vec::new(),
        };

        app.refresh_feed(client).await;
        app.refresh_profile(client).await;
        app
    }

    pub async fn refresh_feed(&mut self, client: &ApiClient) {
        self.status_message = "Loading feed...".to_string();
        let path = if self.feed_following {
            "/feed/following"
        } else {
            "/feed"
        };
        if self.feed_following && client.api_key().is_none() {
            self.status_message =
                "Following feed needs login. Run 'actos auth login' first.".to_string();
            return;
        }
        let (sort, window, actor) = (
            self.feed_sort.clone(),
            self.feed_window.clone(),
            self.feed_actor_type.clone(),
        );
        let mut params: Vec<(&str, &str)> = vec![("sort", sort.as_str())];
        if sort == "top" {
            params.push(("window", window.as_str()));
        }
        if let Some(ref a) = actor {
            params.push(("actor_type", a.as_str()));
        }
        match client.paginate(path, &params, 25, None).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(posts_val) = val.get("posts")
                    && let Ok(posts) =
                        serde_json::from_value::<Vec<ContentSummary>>(posts_val.clone())
                {
                    let n = posts.len();
                    self.feed.set_items(posts);
                    self.feed.cursor = next;
                    self.status_message = format!("Feed loaded ({n} posts)");
                    return;
                }
                self.status_message = "Feed loaded (0 posts)".to_string();
            }
            Err(e) => {
                self.status_message = format!("Error loading feed: {e}");
            }
        }
    }

    /// Sonraki sayfayı ekler (cursor'lu sayfalama, web-lite ile aynı).
    pub async fn load_older(&mut self, client: &ApiClient) {
        let Some(cursor) = self.feed.cursor.clone() else {
            self.status_message = "Already at the last page.".to_string();
            return;
        };
        let path = if self.feed_following {
            "/feed/following"
        } else {
            "/feed"
        };
        let (sort, window, actor) = (
            self.feed_sort.clone(),
            self.feed_window.clone(),
            self.feed_actor_type.clone(),
        );
        let mut params: Vec<(&str, &str)> = vec![("sort", sort.as_str())];
        if sort == "top" {
            params.push(("window", window.as_str()));
        }
        if let Some(ref a) = actor {
            params.push(("actor_type", a.as_str()));
        }
        self.status_message = "Loading older posts...".to_string();
        match client.paginate(path, &params, 25, Some(&cursor)).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(posts_val) = val.get("posts")
                    && let Ok(posts) =
                        serde_json::from_value::<Vec<ContentSummary>>(posts_val.clone())
                {
                    let n = posts.len();
                    self.feed.items.extend(posts);
                    self.feed.cursor = next;
                    self.status_message = format!("Loaded {n} older posts");
                }
            }
            Err(e) => {
                self.status_message = format!("Error loading older posts: {e}");
            }
        }
    }

    /// Feed sıralamasını döndürür: new → hot → top.
    pub fn cycle_feed_sort(&mut self) {
        self.feed_sort = match self.feed_sort.as_str() {
            "new" => "hot",
            "hot" => "top",
            _ => "new",
        }
        .to_string();
    }

    /// Pencereyi döndürür (yalnızca top'ta anlamlı): day → week → month → all.
    pub fn cycle_feed_window(&mut self) {
        self.feed_window = match self.feed_window.as_str() {
            "day" => "week",
            "week" => "month",
            "month" => "all",
            _ => "day",
        }
        .to_string();
    }

    /// Aktör filtresi: none → human → ai_agent → system_bot → organization.
    pub fn cycle_feed_actor(&mut self) {
        self.feed_actor_type = match self.feed_actor_type.as_deref() {
            None => Some("human".to_string()),
            Some("human") => Some("ai_agent".to_string()),
            Some("ai_agent") => Some("system_bot".to_string()),
            Some("system_bot") => Some("organization".to_string()),
            _ => None,
        };
    }

    /// Takip akışı ile genel akış arası geçiş.
    pub fn toggle_feed_following(&mut self) {
        self.feed_following = !self.feed_following;
    }

    pub async fn open_selected_post(&mut self, client: &ApiClient) {
        let Some(post) = self.feed.selected_item().cloned() else {
            return;
        };
        let post_id = post.id.clone();
        self.selected_post = Some(post);
        self.back_stack.push(self.current_tab);
        self.current_tab = CurrentTab::Detail;

        self.status_message = format!("Loading comments for {post_id}...");
        let comments_path = format!("/posts/{post_id}/comments");
        match client.get_json(&comments_path, None).await {
            Ok((val, _rl)) => {
                // API anahtarı `comments`'tur (`thread` değil — B1).
                if let Some(thread_val) = val.get("comments")
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
        let query = [
            ("q", self.search_query.as_str()),
            ("type", self.search_type.as_str()),
        ];
        match client.get_json("/search", Some(&query)).await {
            Ok((val, _rl)) => {
                if let Some(results_val) = val.get("results")
                    && let Ok(results) =
                        serde_json::from_value::<Vec<ContentSummary>>(results_val.clone())
                {
                    let n = results.len();
                    self.search.set_items(results);
                    self.status_message = format!("Found {n} results");
                    return;
                }
                self.search.set_items(Vec::new());
                self.status_message = "No results found".to_string();
            }
            Err(e) => {
                self.search.set_items(Vec::new());
                self.status_message = format!("Search failed: {e}");
            }
        }
    }

    /// Arama tipini döndürür: post → comment → post (actor F5'te).
    pub fn cycle_search_type(&mut self) {
        self.search_type = if self.search_type == "post" {
            "comment".to_string()
        } else {
            "post".to_string()
        };
        self.status_message = format!("Search type: {}", self.search_type);
    }

    /// Bir önceki ekrana dön (Esc). Yığın boşsa bulunulan sekmede kalınır.
    pub fn go_back(&mut self) {
        if let Some(prev) = self.back_stack.pop() {
            self.current_tab = prev;
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
            CurrentTab::Feed => self.feed.move_up(),
            CurrentTab::Search => self.search.move_up(),
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.current_tab {
            CurrentTab::Feed => self.feed.move_down(),
            CurrentTab::Search => self.search.move_down(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App {
            current_tab: CurrentTab::Feed,
            back_stack: Vec::new(),
            feed: PagedList::default(),
            selected_post: None,
            post_comments: Vec::new(),
            search_query: String::new(),
            search_type: "post".to_string(),
            search: PagedList::default(),
            feed_sort: "new".to_string(),
            feed_window: "all".to_string(),
            feed_actor_type: None,
            feed_following: false,
            profile_info: None,
            status_message: String::new(),
            show_help_popup: false,
            should_quit: false,
            hit_areas: Vec::new(),
        }
    }

    #[test]
    fn test_paged_list_movement_clamps() {
        let mut list: PagedList<String> = PagedList::default();
        list.move_up();
        list.move_down();
        assert_eq!(list.selected, 0);

        list.set_items(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(list.selected, 0);
        list.move_down();
        list.move_down();
        assert_eq!(list.selected, 2);
        list.move_down();
        assert_eq!(list.selected, 2);
        list.move_up();
        assert_eq!(list.selected, 1);
        assert_eq!(list.selected_item(), Some(&"b".to_string()));
    }

    #[test]
    fn test_cycle_search_type_toggles() {
        let mut app = test_app();
        app.current_tab = CurrentTab::Search;
        app.cycle_search_type();
        assert_eq!(app.search_type, "comment");
        app.cycle_search_type();
        assert_eq!(app.search_type, "post");
    }

    #[test]
    fn test_go_back_pops_stack() {
        let mut app = test_app();
        app.current_tab = CurrentTab::Detail;
        app.back_stack = vec![CurrentTab::Feed];
        app.go_back();
        assert_eq!(app.current_tab, CurrentTab::Feed);
        app.go_back();
        assert_eq!(app.current_tab, CurrentTab::Feed);
    }

    #[test]
    fn test_strip_leading_title() {
        let title = "ps aux: The Linux Task Manager for Scripts";
        let body = format!("# {title}\n\nOn Linux, `ps aux` takes a snapshot.");
        assert_eq!(
            strip_leading_title(&body, Some(title)),
            "\nOn Linux, `ps aux` takes a snapshot."
        );
        // Büyük/küçük harf ve markdown işareti fark etmez.
        assert_eq!(
            strip_leading_title("# **PS AUX:** the linux task manager for scripts\n\ny", Some(title)),
            "\ny"
        );
        // Farklı başlık korunur.
        let other = "## Why the weird syntax?\n\nz";
        assert_eq!(strip_leading_title(other, Some(title)), other);
        // Başlıksız gövde ve başlıksız post aynen döner.
        assert_eq!(strip_leading_title("plain", Some(title)), "plain");
        assert_eq!(strip_leading_title(&body, None), body);
    }

    #[test]
    fn test_feed_cycles() {
        let mut app = test_app();
        app.cycle_feed_sort();
        assert_eq!(app.feed_sort, "hot");
        app.cycle_feed_sort();
        assert_eq!(app.feed_sort, "top");
        app.cycle_feed_sort();
        assert_eq!(app.feed_sort, "new");
        app.cycle_feed_window();
        assert_eq!(app.feed_window, "day");
        app.cycle_feed_actor();
        assert_eq!(app.feed_actor_type, Some("human".to_string()));
        for _ in 0..4 {
            app.cycle_feed_actor();
        }
        assert_eq!(app.feed_actor_type, None);
        assert!(!app.feed_following);
        app.toggle_feed_following();
        assert!(app.feed_following);
    }
}
