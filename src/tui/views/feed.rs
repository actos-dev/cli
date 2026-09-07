use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;
use crate::tui::mouse::MouseAction;

pub fn render_feed(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.feed_posts.is_empty() {
        let empty_block = Block::default()
            .title(" Feed (Empty) ")
            .borders(Borders::ALL);
        frame.render_widget(empty_block, area);
        return;
    }

    let items: Vec<ListItem> = app
        .feed_posts
        .iter()
        .enumerate()
        .map(|(idx, post)| {
            let is_selected = idx == app.feed_selected;
            let prefix = if is_selected { "▶ " } else { "  " };

            let title_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let score_style = Style::default().fg(Color::Green);
            let author_style = Style::default().fg(Color::Cyan);

            let title_str = post.title.as_deref().unwrap_or("(no title)");
            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(format!("{:<40}", title_str), title_style),
                Span::raw("  by "),
                Span::styled(format!("@{}", post.author.username), author_style),
                Span::raw("  [Score: "),
                Span::styled(format!("{}", post.score), score_style),
                Span::raw(format!(", Comments: {}]", post.comment_count)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Feed (Public Stream) ")
            .borders(Borders::ALL),
    );

    frame.render_widget(list, area);

    // Satır tıklama alanları: çerçeve içi, satır başına bir hücre.
    let list_top = area.y.saturating_add(1);
    let visible = (area.height.saturating_sub(2)) as usize;
    for (idx, _) in app.feed_posts.iter().enumerate().take(visible) {
        app.hit_areas.push((
            Rect::new(
                area.x.saturating_add(1),
                list_top.saturating_add(idx as u16),
                area.width.saturating_sub(2),
                1,
            ),
            MouseAction::SelectFeed(idx),
        ));
    }
}
