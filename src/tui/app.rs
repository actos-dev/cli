use crate::client::ApiClient;
use crate::tui::mouse::MouseAction;
use actos_sdk::actos_types::actor::ActorProfileResponse;
use actos_sdk::actos_types::auth::ActorSummary;
use actos_sdk::actos_types::content::{CommentNodeResponse, ContentSummary};
use actos_sdk::actos_types::notification::NotificationSummary;
use actos_sdk::actos_types::tag::TagSummary;
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentTab {
    Feed,
    Tags,
    TagPosts,
    Actors,
    Search,
    Inbox,
    Saves,
    Profile,
    Detail,
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

/// Detail üstü giriş kutuları: yanıt, bildirim ve silme onayı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    Reply { post_id: String, text: String },
    Report { target_id: String, text: String },
    ConfirmDeletePost { post_id: String },
}

impl Overlay {
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::Reply { .. } => " Reply (Enter: send, Esc: cancel) ",
            Self::Report { .. } => " Report post (Enter: send, Esc: cancel) ",
            Self::ConfirmDeletePost { .. } => " Delete post? ",
        }
    }

    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Reply { text, .. } | Self::Report { text, .. } => Some(text),
            Self::ConfirmDeletePost { .. } => None,
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self {
            Self::Reply { text, .. } | Self::Report { text, .. } => text.push(c),
            Self::ConfirmDeletePost { .. } => {}
        }
    }

    pub fn pop_char(&mut self) {
        match self {
            Self::Reply { text, .. } | Self::Report { text, .. } => {
                text.pop();
            }
            Self::ConfirmDeletePost { .. } => {}
        }
    }
}

pub struct App {
    pub current_tab: CurrentTab,
    /// Geri yığını: Detail gibi üst ekranlara buradan dönülür (Esc).
    pub back_stack: Vec<CurrentTab>,
    pub feed: PagedList<ContentSummary>,
    pub selected_post: Option<ContentSummary>,
    pub post_comments: Vec<CommentNodeResponse>,
    pub detail_scroll: u16,
    pub detail_lines: u16,
    pub overlay: Option<Overlay>,
    pub search_query: String,
    pub search_type: String,
    pub search: PagedList<ContentSummary>,
    pub feed_sort: String,
    pub feed_window: String,
    pub feed_actor_type: Option<String>,
    pub feed_following: bool,
    pub tags: PagedList<TagSummary>,
    pub tag_posts_name: String,
    pub tag_posts_sort: String,
    pub tag_posts: PagedList<ContentSummary>,
    pub actors: PagedList<ActorSummary>,
    pub actors_type: Option<String>,
    pub inbox: PagedList<NotificationSummary>,
    pub inbox_unread_only: bool,
    pub inbox_unread_total: i64,
    pub saves: PagedList<ContentSummary>,
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
            detail_scroll: 0,
            detail_lines: 0,
            overlay: None,
            search_query: String::new(),
            search_type: "post".to_string(),
            search: PagedList::default(),
            feed_sort: "new".to_string(),
            feed_window: "all".to_string(),
            feed_actor_type: None,
            feed_following: false,
            tags: PagedList::default(),
            tag_posts_name: String::new(),
            tag_posts_sort: "new".to_string(),
            tag_posts: PagedList::default(),
            actors: PagedList::default(),
            actors_type: None,
            inbox: PagedList::default(),
            inbox_unread_only: false,
            inbox_unread_total: 0,
            saves: PagedList::default(),
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

    // ---------- Tags ----------

    pub async fn refresh_tags(&mut self, client: &ApiClient) {
        self.status_message = "Loading tags...".to_string();
        match client.paginate("/tags", &[], 25, None).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("tags")
                    && let Ok(tags) =
                        serde_json::from_value::<Vec<TagSummary>>(list.clone())
                {
                    let n = tags.len();
                    self.tags.set_items(tags);
                    self.tags.cursor = next;
                    self.status_message = format!("Tags loaded ({n} tags)");
                    return;
                }
                self.status_message = "Tags loaded (0 tags)".to_string();
            }
            Err(e) => self.status_message = format!("Error loading tags: {e}"),
        }
    }

    pub async fn load_tags_older(&mut self, client: &ApiClient) {
        let Some(cursor) = self.tags.cursor.clone() else {
            self.status_message = "Already at the last page.".to_string();
            return;
        };
        match client.paginate("/tags", &[], 25, Some(&cursor)).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("tags")
                    && let Ok(tags) =
                        serde_json::from_value::<Vec<TagSummary>>(list.clone())
                {
                    self.tags.items.extend(tags);
                    self.tags.cursor = next;
                    self.status_message = "Loaded more tags.".to_string();
                }
            }
            Err(e) => self.status_message = format!("Error loading tags: {e}"),
        }
    }

    pub async fn open_tag_posts(&mut self, client: &ApiClient) {
        let Some(tag) = self.tags.selected_item() else {
            return;
        };
        self.tag_posts_name = tag.name.clone();
        self.tag_posts_sort = "new".to_string();
        self.back_stack.push(self.current_tab);
        self.current_tab = CurrentTab::TagPosts;
        self.refresh_tag_posts(client).await;
    }

    pub async fn refresh_tag_posts(&mut self, client: &ApiClient) {
        let name = self.tag_posts_name.clone();
        let sort = self.tag_posts_sort.clone();
        self.status_message = format!("Loading #{name}...");
        let path = format!("/tags/{name}/posts");
        let params = [("sort", sort.as_str())];
        match client.paginate(&path, &params, 25, None).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("posts")
                    && let Ok(posts) =
                        serde_json::from_value::<Vec<ContentSummary>>(list.clone())
                {
                    let n = posts.len();
                    self.tag_posts.set_items(posts);
                    self.tag_posts.cursor = next;
                    self.status_message = format!("#{name}: {n} posts");
                    return;
                }
                self.status_message = format!("#{name}: no live posts.");
            }
            Err(e) => self.status_message = format!("Error loading tag: {e}"),
        }
    }

    pub async fn load_tag_posts_older(&mut self, client: &ApiClient) {
        let Some(cursor) = self.tag_posts.cursor.clone() else {
            self.status_message = "Already at the last page.".to_string();
            return;
        };
        let name = self.tag_posts_name.clone();
        let sort = self.tag_posts_sort.clone();
        let path = format!("/tags/{name}/posts");
        let params = [("sort", sort.as_str())];
        match client.paginate(&path, &params, 25, Some(&cursor)).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("posts")
                    && let Ok(posts) =
                        serde_json::from_value::<Vec<ContentSummary>>(list.clone())
                {
                    self.tag_posts.items.extend(posts);
                    self.tag_posts.cursor = next;
                    self.status_message = format!("Loaded more #{name} posts.");
                }
            }
            Err(e) => self.status_message = format!("Error loading tag: {e}"),
        }
    }

    pub fn cycle_tag_posts_sort(&mut self) {
        self.tag_posts_sort = match self.tag_posts_sort.as_str() {
            "new" => "top",
            "top" => "hot",
            _ => "new",
        }
        .to_string();
    }

    pub async fn open_selected_tag_post(&mut self, client: &ApiClient) {
        let Some(post) = self.tag_posts.selected_item().cloned() else {
            return;
        };
        self.open_post_detail(client, post).await;
    }

    // ---------- Actors ----------

    pub async fn refresh_actors(&mut self, client: &ApiClient) {
        self.status_message = "Loading actors...".to_string();
        let actor_type = self.actors_type.clone();
        let mut params: Vec<(&str, &str)> = vec![("sort", "new")];
        if let Some(ref t) = actor_type {
            params.push(("type", t.as_str()));
        }
        match client.paginate("/actors", &params, 25, None).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("actors")
                    && let Ok(actors) =
                        serde_json::from_value::<Vec<ActorSummary>>(list.clone())
                {
                    let n = actors.len();
                    self.actors.set_items(actors);
                    self.actors.cursor = next;
                    self.status_message = format!("Actors loaded ({n})");
                    return;
                }
                self.status_message = "Actors loaded (0).".to_string();
            }
            Err(e) => self.status_message = format!("Error loading actors: {e}"),
        }
    }

    pub fn cycle_actors_type(&mut self) {
        self.actors_type = match self.actors_type.as_deref() {
            None => Some("human".to_string()),
            Some("human") => Some("ai_agent".to_string()),
            Some("ai_agent") => Some("system_bot".to_string()),
            Some("system_bot") => Some("organization".to_string()),
            _ => None,
        };
    }

    // ---------- Inbox ----------

    pub async fn refresh_inbox(&mut self, client: &ApiClient) {
        if client.api_key().is_none() {
            self.status_message =
                "Inbox needs login. Run 'actos auth login' first.".to_string();
            return;
        }
        self.status_message = "Loading inbox...".to_string();
        let mut params: Vec<(&str, &str)> = Vec::new();
        if self.inbox_unread_only {
            params.push(("unread", "true"));
        }
        match client.paginate("/me/inbox", &params, 25, None).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let total = val.get("unread_count").and_then(|v| v.as_i64()).unwrap_or(0);
                self.inbox_unread_total = total;
                if let Some(list) = val.get("notifications")
                    && let Ok(items) =
                        serde_json::from_value::<Vec<NotificationSummary>>(list.clone())
                {
                    let n = items.len();
                    self.inbox.set_items(items);
                    self.inbox.cursor = next;
                    self.status_message = format!("Inbox ({total} unread total, {n} shown)");
                    return;
                }
                self.status_message = "Inbox is empty.".to_string();
            }
            Err(e) => self.status_message = format!("Error loading inbox: {e}"),
        }
    }

    pub async fn load_inbox_older(&mut self, client: &ApiClient) {
        let Some(cursor) = self.inbox.cursor.clone() else {
            self.status_message = "Already at the last page.".to_string();
            return;
        };
        let mut params: Vec<(&str, &str)> = Vec::new();
        if self.inbox_unread_only {
            params.push(("unread", "true"));
        }
        match client.paginate("/me/inbox", &params, 25, Some(&cursor)).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("notifications")
                    && let Ok(items) =
                        serde_json::from_value::<Vec<NotificationSummary>>(list.clone())
                {
                    self.inbox.items.extend(items);
                    self.inbox.cursor = next;
                    self.status_message = "Loaded older notifications.".to_string();
                }
            }
            Err(e) => self.status_message = format!("Error loading inbox: {e}"),
        }
    }

    pub fn toggle_inbox_unread(&mut self) {
        self.inbox_unread_only = !self.inbox_unread_only;
    }

    pub async fn mark_selected_read(&mut self, client: &ApiClient) {
        let Some(notif) = self.inbox.selected_item() else {
            return;
        };
        if notif.read_at.is_some() {
            self.status_message = "Already read.".to_string();
            return;
        }
        let id = notif.id.clone();
        match client
            .execute_request(
                reqwest::Method::PATCH,
                &format!("/me/inbox/{id}/read"),
                None,
                None,
                None,
            )
            .await
        {
            Ok(_) => {
                if let Some(item) = self.inbox.items.iter_mut().find(|n| n.id == id) {
                    item.read_at = Some(String::new());
                }
                self.inbox_unread_total = self.inbox_unread_total.saturating_sub(1);
                self.status_message = "Marked read.".to_string();
            }
            Err(e) => self.status_message = format!("Mark read failed: {e}"),
        }
    }

    pub async fn mark_all_read(&mut self, client: &ApiClient) {
        match client
            .post_json("/me/inbox/read", &serde_json::json!({}), None)
            .await
        {
            Ok((val, _)) => {
                let marked = val.get("marked").and_then(|v| v.as_i64()).unwrap_or(0);
                for item in self.inbox.items.iter_mut() {
                    item.read_at = Some(String::new());
                }
                self.inbox_unread_total = 0;
                self.status_message = format!("Marked {marked} notification(s) read.");
            }
            Err(e) => self.status_message = format!("Mark all failed: {e}"),
        }
    }

    pub async fn open_selected_notification(&mut self, client: &ApiClient) {
        let Some(notif) = self.inbox.selected_item() else {
            return;
        };
        if notif.target_type != "content" {
            self.status_message = "Actor notifications open in F5 (profiles).".to_string();
            return;
        }
        let target = notif.target_id.clone();
        self.open_content(client, &target).await;
    }

    // ---------- Saves ----------

    pub async fn refresh_saves(&mut self, client: &ApiClient) {
        if client.api_key().is_none() {
            self.status_message =
                "Saves need login. Run 'actos auth login' first.".to_string();
            return;
        }
        self.status_message = "Loading saves...".to_string();
        match client.paginate("/me/saves", &[], 25, None).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("saves")
                    && let Ok(items) =
                        serde_json::from_value::<Vec<ContentSummary>>(list.clone())
                {
                    let n = items.len();
                    self.saves.set_items(items);
                    self.saves.cursor = next;
                    self.status_message = format!("Saves loaded ({n})");
                    return;
                }
                self.status_message = "Nothing saved yet.".to_string();
            }
            Err(e) => self.status_message = format!("Error loading saves: {e}"),
        }
    }

    pub async fn load_saves_older(&mut self, client: &ApiClient) {
        let Some(cursor) = self.saves.cursor.clone() else {
            self.status_message = "Already at the last page.".to_string();
            return;
        };
        match client.paginate("/me/saves", &[], 25, Some(&cursor)).await {
            Ok((val, _rl)) => {
                let next = val
                    .get("next_cursor")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if let Some(list) = val.get("saves")
                    && let Ok(items) =
                        serde_json::from_value::<Vec<ContentSummary>>(list.clone())
                {
                    self.saves.items.extend(items);
                    self.saves.cursor = next;
                    self.status_message = "Loaded older saves.".to_string();
                }
            }
            Err(e) => self.status_message = format!("Error loading saves: {e}"),
        }
    }

    pub async fn open_selected_save(&mut self, client: &ApiClient) {
        let Some(item) = self.saves.selected_item() else {
            return;
        };
        let id = item.id.clone();
        self.open_content(client, &id).await;
    }

    /// İçeriği türüne göre açar: post doğrudan, yorum ata zincirinden
    /// kök post'a giderek (web-lite kuralı).
    pub async fn open_content(&mut self, client: &ApiClient, id: &str) {
        if let Ok((val, _)) = client.get_json(&format!("/posts/{id}"), None).await
            && let Ok(post) = serde_json::from_value::<ContentSummary>(val)
        {
            self.open_post_detail(client, post).await;
            return;
        }
        if let Ok((val, _)) = client.get_json(&format!("/comments/{id}"), None).await
            && let Some(root) = val
                .get("ancestors")
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
            && let Some(root_id) = root.get("id").and_then(|v| v.as_str())
        {
            let root_id = root_id.to_string();
            if let Ok((pval, _)) = client.get_json(&format!("/posts/{root_id}"), None).await
                && let Ok(post) = serde_json::from_value::<ContentSummary>(pval)
            {
                self.open_post_detail(client, post).await;
                return;
            }
        }
        self.status_message = format!("Could not open {id} (deleted or missing).");
    }

    /// Seçili postu Detail yığınına açar (yorumlarla birlikte).
    pub async fn open_post_detail(&mut self, client: &ApiClient, post: ContentSummary) {
        let post_id = post.id.clone();
        self.selected_post = Some(post);
        self.detail_scroll = 0;
        self.detail_lines = 0;
        self.overlay = None;
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

    pub async fn open_selected_post(&mut self, client: &ApiClient) {
        let Some(post) = self.feed.selected_item().cloned() else {
            return;
        };
        self.open_post_detail(client, post).await;
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

    /// Detail kaydırması (j/k, PgUp/PgDn, tekerlek). Alta taşmaya izin verilmez.
    pub fn scroll_detail_by(&mut self, delta: i16) {
        let next = self.detail_scroll as i16 + delta;
        let max = self.detail_lines as i16;
        self.detail_scroll = next.clamp(0, max.max(0)) as u16;
    }

    fn require_detail_post(&self) -> Option<String> {
        self.selected_post.as_ref().map(|p| p.id.clone())
    }

    fn require_login(client: &ApiClient) -> Result<(), String> {
        if client.api_key().is_none() {
            Err("Login required. Run 'actos auth login' first.".to_string())
        } else {
            Ok(())
        }
    }

    pub fn open_overlay_reply(&mut self, client: &ApiClient) {
        match (Self::require_login(client), self.require_detail_post()) {
            (Ok(()), Some(post_id)) => {
                self.overlay = Some(Overlay::Reply {
                    post_id,
                    text: String::new(),
                });
            }
            (Err(msg), _) => self.status_message = msg,
            _ => {}
        }
    }

    pub fn open_overlay_report(&mut self, client: &ApiClient) {
        match (Self::require_login(client), self.require_detail_post()) {
            (Ok(()), Some(target_id)) => {
                self.overlay = Some(Overlay::Report {
                    target_id,
                    text: String::new(),
                });
            }
            (Err(msg), _) => self.status_message = msg,
            _ => {}
        }
    }

    pub fn open_confirm_delete(&mut self, client: &ApiClient) {
        match (Self::require_login(client), self.require_detail_post()) {
            (Ok(()), Some(post_id)) => {
                self.overlay = Some(Overlay::ConfirmDeletePost { post_id });
            }
            (Err(msg), _) => self.status_message = msg,
            _ => {}
        }
    }

    /// Açık overlay'i çalıştırır (Enter). Boş metin gönderilmez.
    pub async fn submit_overlay(&mut self, client: &ApiClient) {
        let Some(overlay) = self.overlay.clone() else {
            return;
        };
        match overlay {
            Overlay::Reply { post_id, text } => {
                if text.trim().is_empty() {
                    self.status_message = "Reply is empty — nothing sent.".to_string();
                    return;
                }
                let body = serde_json::json!({ "body": text, "parent_id": null });
                match client
                    .post_json(&format!("/posts/{post_id}/comments"), &body, None)
                    .await
                {
                    Ok(_) => {
                        self.overlay = None;
                        self.status_message = "Reply posted.".to_string();
                        self.reload_comments(client).await;
                    }
                    Err(e) => self.status_message = format!("Reply failed: {e}"),
                }
            }
            Overlay::Report { target_id, text } => {
                if text.trim().is_empty() {
                    self.status_message = "Reason is empty — nothing sent.".to_string();
                    return;
                }
                let body = serde_json::json!({
                    "target_id": target_id,
                    "target_type": "post",
                    "reason": text,
                });
                match client.post_json("/reports", &body, None).await {
                    Ok(_) => {
                        self.overlay = None;
                        self.status_message = "Report sent. Thanks.".to_string();
                    }
                    Err(e) => self.status_message = format!("Report failed: {e}"),
                }
            }
            Overlay::ConfirmDeletePost { post_id } => {
                match client
                    .execute_request(
                        reqwest::Method::DELETE,
                        &format!("/posts/{post_id}"),
                        None,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(_) => {
                        self.overlay = None;
                        self.selected_post = None;
                        self.status_message = "Post deleted.".to_string();
                        self.go_back();
                        self.refresh_feed(client).await;
                    }
                    Err(e) => self.status_message = format!("Delete failed: {e}"),
                }
            }
        }
    }

    async fn reload_comments(&mut self, client: &ApiClient) {
        let Some(post_id) = self.require_detail_post() else {
            return;
        };
        match client
            .get_json(&format!("/posts/{post_id}/comments"), None)
            .await
        {
            Ok((val, _rl)) => {
                if let Some(list) = val.get("comments")
                    && let Ok(comments) =
                        serde_json::from_value::<Vec<CommentNodeResponse>>(list.clone())
                {
                    self.post_comments = comments;
                }
            }
            Err(e) => self.status_message = format!("Could not reload comments: {e}"),
        }
    }

    /// Oya ver (+1/-1/0). Skor yanıttan tazelenir.
    pub async fn vote_current(&mut self, client: &ApiClient, value: i32) {
        if let Err(msg) = Self::require_login(client) {
            self.status_message = msg;
            return;
        }
        let Some(post_id) = self.require_detail_post() else {
            return;
        };
        let body = serde_json::json!({ "value": value });
        match client
            .execute_request(
                reqwest::Method::PUT,
                &format!("/contents/{post_id}/vote"),
                None,
                Some(body.to_string().into_bytes()),
                None,
            )
            .await
        {
            Ok((_, _, bytes, _)) => {
                if let Ok(res) = serde_json::from_slice::<serde_json::Value>(&bytes)
                    && let Some(score) = res.get("score").and_then(|s| s.as_i64())
                    && let Some(post) = self.selected_post.as_mut()
                {
                    post.score = score as i32;
                }
                self.status_message = if value == 0 {
                    "Vote retracted.".to_string()
                } else {
                    "Voted.".to_string()
                };
            }
            Err(e) => self.status_message = format!("Vote failed: {e}"),
        }
    }

    /// Kaydet/kaydı kaldır (iki ayrı uç, durum tutulmadan).
    pub async fn save_current(&mut self, client: &ApiClient, save: bool) {
        if let Err(msg) = Self::require_login(client) {
            self.status_message = msg;
            return;
        }
        let Some(post_id) = self.require_detail_post() else {
            return;
        };
        let method = if save {
            reqwest::Method::PUT
        } else {
            reqwest::Method::DELETE
        };
        match client
            .execute_request(
                method,
                &format!("/contents/{post_id}/save"),
                None,
                None,
                None,
            )
            .await
        {
            Ok(_) => {
                self.status_message = if save {
                    "Saved.".to_string()
                } else {
                    "Unsaved.".to_string()
                };
            }
            Err(e) => self.status_message = format!("Save failed: {e}"),
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
            CurrentTab::Feed => CurrentTab::Tags,
            CurrentTab::Tags | CurrentTab::TagPosts => CurrentTab::Actors,
            CurrentTab::Actors => CurrentTab::Search,
            CurrentTab::Search => CurrentTab::Inbox,
            CurrentTab::Inbox => CurrentTab::Saves,
            CurrentTab::Saves => CurrentTab::Profile,
            CurrentTab::Profile => CurrentTab::Help,
            CurrentTab::Help => CurrentTab::Feed,
            CurrentTab::Detail => CurrentTab::Feed,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            CurrentTab::Feed => CurrentTab::Help,
            CurrentTab::Tags | CurrentTab::TagPosts => CurrentTab::Feed,
            CurrentTab::Actors => CurrentTab::Tags,
            CurrentTab::Search => CurrentTab::Actors,
            CurrentTab::Inbox => CurrentTab::Search,
            CurrentTab::Saves => CurrentTab::Inbox,
            CurrentTab::Profile => CurrentTab::Saves,
            CurrentTab::Help => CurrentTab::Profile,
            CurrentTab::Detail => CurrentTab::Feed,
        };
    }

    pub fn move_up(&mut self) {
        match self.current_tab {
            CurrentTab::Feed => self.feed.move_up(),
            CurrentTab::Tags => self.tags.move_up(),
            CurrentTab::TagPosts => self.tag_posts.move_up(),
            CurrentTab::Actors => self.actors.move_up(),
            CurrentTab::Search => self.search.move_up(),
            CurrentTab::Inbox => self.inbox.move_up(),
            CurrentTab::Saves => self.saves.move_up(),
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.current_tab {
            CurrentTab::Feed => self.feed.move_down(),
            CurrentTab::Tags => self.tags.move_down(),
            CurrentTab::TagPosts => self.tag_posts.move_down(),
            CurrentTab::Actors => self.actors.move_down(),
            CurrentTab::Search => self.search.move_down(),
            CurrentTab::Inbox => self.inbox.move_down(),
            CurrentTab::Saves => self.saves.move_down(),
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
            detail_scroll: 0,
            detail_lines: 0,
            overlay: None,
            search_query: String::new(),
            search_type: "post".to_string(),
            search: PagedList::default(),
            feed_sort: "new".to_string(),
            feed_window: "all".to_string(),
            feed_actor_type: None,
            feed_following: false,
            tags: PagedList::default(),
            tag_posts_name: String::new(),
            tag_posts_sort: "new".to_string(),
            tag_posts: PagedList::default(),
            actors: PagedList::default(),
            actors_type: None,
            inbox: PagedList::default(),
            inbox_unread_only: false,
            inbox_unread_total: 0,
            saves: PagedList::default(),
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

    fn test_client(key: Option<&str>) -> crate::client::ApiClient {
        crate::client::ApiClient::new(
            "https://api.actos.test".to_string(),
            key.map(ToString::to_string),
            30,
            false,
            false,
        )
        .expect("test client builds")
    }

    fn test_post() -> ContentSummary {
        serde_json::from_value(serde_json::json!({
            "id": "c_test1",
            "content_type": "post",
            "author": {
                "id": "a_1", "username": "alice", "actor_type": "human",
                "display_name": null, "bio": null,
                "created_at": "2026-01-01T00:00:00Z",
                "trust_level": 0, "avatar_url": null
            },
            "author_deleted": false,
            "title": "Hello",
            "body": "world",
            "body_format": "markdown",
            "body_html": null,
            "metadata": {},
            "tags": [],
            "score": 5, "upvotes": 5, "downvotes": 0,
            "comment_count": 0,
            "created_at": "2026-01-01T00:00:00Z",
            "edited_at": null,
            "attachments": null,
            "deleted": false
        }))
        .expect("fixture parses")
    }

    #[test]
    fn test_scroll_detail_clamps() {
        let mut app = test_app();
        app.detail_lines = 10;
        app.scroll_detail_by(100);
        assert_eq!(app.detail_scroll, 10);
        app.scroll_detail_by(-100);
        assert_eq!(app.detail_scroll, 0);
        app.scroll_detail_by(3);
        assert_eq!(app.detail_scroll, 3);
    }

    #[test]
    fn test_overlay_text_editing() {
        let mut o = Overlay::Reply {
            post_id: "c_1".to_string(),
            text: String::new(),
        };
        assert_eq!(o.title(), " Reply (Enter: send, Esc: cancel) ");
        o.push_char('h');
        o.push_char('i');
        assert_eq!(o.text(), Some("hi"));
        o.pop_char();
        assert_eq!(o.text(), Some("h"));
        let c = Overlay::ConfirmDeletePost {
            post_id: "c_1".to_string(),
        };
        assert_eq!(c.text(), None);
    }

    #[test]
    fn test_overlay_requires_login() {
        let client = test_client(None);
        let mut app = test_app();
        app.selected_post = Some(test_post());
        app.open_overlay_reply(&client);
        assert!(app.overlay.is_none());
        assert!(app.status_message.contains("Login required"));
        app.open_overlay_report(&client);
        assert!(app.overlay.is_none());
        app.open_confirm_delete(&client);
        assert!(app.overlay.is_none());
    }

    #[test]
    fn test_overlay_opens_with_login_and_post() {
        let client = test_client(Some("actos_test"));
        let mut app = test_app();
        app.open_overlay_reply(&client);
        assert!(app.overlay.is_none());
        app.selected_post = Some(test_post());
        app.open_overlay_reply(&client);
        assert!(matches!(app.overlay, Some(Overlay::Reply { .. })));
        app.open_overlay_report(&client);
        assert!(matches!(app.overlay, Some(Overlay::Report { .. })));
        app.open_confirm_delete(&client);
        assert!(matches!(app.overlay, Some(Overlay::ConfirmDeletePost { .. })));
    }

    #[tokio::test]
    async fn test_submit_empty_overlay_sends_nothing() {
        let client = test_client(Some("actos_test"));
        let mut app = test_app();
        app.selected_post = Some(test_post());
        app.overlay = Some(Overlay::Reply {
            post_id: "c_test1".to_string(),
            text: "   ".to_string(),
        });
        app.submit_overlay(&client).await;
        assert!(app.overlay.is_some());
        assert!(app.status_message.contains("empty"));
    }

    #[tokio::test]
    async fn test_refresh_tags_parses_mock() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/tags"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "tags": [
                        {"name": "rust", "post_count": 12,
                         "created_at": "2026-01-01T00:00:00Z"}
                    ],
                    "next_cursor": null
                }),
            ))
            .mount(&server)
            .await;
        let client = crate::client::ApiClient::new(
            server.uri(),
            None,
            30,
            false,
            false,
        )
        .expect("mock client builds");
        let mut app = test_app();
        app.refresh_tags(&client).await;
        assert_eq!(app.tags.items.len(), 1);
        assert_eq!(app.tags.items[0].name, "rust");
        assert_eq!(app.tags.cursor, None);
    }

    #[tokio::test]
    async fn test_refresh_inbox_parses_mock() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/me/inbox"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "notifications": [
                        {"id": "n_1", "kind": "comment_on_post",
                         "actor": null, "target_type": "content",
                         "target_id": "c_9", "payload": {},
                         "created_at": "2026-01-01T00:00:00Z",
                         "read_at": null}
                    ],
                    "next_cursor": null,
                    "unread_count": 7
                }),
            ))
            .mount(&server)
            .await;
        let client = crate::client::ApiClient::new(
            server.uri(),
            Some("actos_test".to_string()),
            30,
            false,
            false,
        )
        .expect("mock client builds");
        let mut app = test_app();
        app.refresh_inbox(&client).await;
        assert_eq!(app.inbox.items.len(), 1);
        assert_eq!(app.inbox_unread_total, 7);
        assert!(app.inbox.items[0].read_at.is_none());
    }
}
