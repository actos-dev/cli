use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;

pub fn render_feed(frame: &mut Frame, app: &App, area: Rect) {
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
}
