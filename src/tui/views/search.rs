use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::tui::app::App;
use crate::tui::mouse::MouseAction;

pub fn render_search(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
        .split(area);

    let input_line = Line::from(vec![
        Span::styled("Query: ", Style::default().fg(Color::Yellow)),
        Span::raw(&app.search_query),
        Span::styled("█", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("  [{}] (Ctrl+T: type)", app.search_type),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let input_widget = Paragraph::new(input_line).block(
        Block::default()
            .title(" Search Posts (Type and press Enter) ")
            .borders(Borders::ALL),
    );

    frame.render_widget(input_widget, chunks[0]);

    if app.search.items.is_empty() {
        let empty = Paragraph::new("No search results to display.")
            .block(Block::default().title(" Results ").borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let selected = app.search.selected;
    let items: Vec<ListItem> = app
        .search
        .items
        .iter()
        .enumerate()
        .map(|(idx, post)| {
            let is_selected = idx == selected;
            let prefix = if is_selected { "▶ " } else { "  " };

            let title_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let title_str = if post.content_type == "comment" {
                // Yorumun başlığı olmaz: gövde özeti göster (web-lite kuralı).
                let flat = post.body.split_whitespace().collect::<Vec<_>>().join(" ");
                let cut: String = flat.chars().take(40).collect();
                if flat.chars().count() > 40 {
                    format!("{cut}…")
                } else {
                    cut
                }
            } else {
                post.title
                    .as_deref()
                    .unwrap_or("(no title)")
                    .to_string()
            };
            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(format!("{:<40}", title_str), title_style),
                Span::raw("  by "),
                Span::styled(
                    format!("@{}", post.author.username),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!("  [Score: {}]", post.score)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Search Results ")
            .borders(Borders::ALL),
    );

    frame.render_widget(list, chunks[1]);

    let list_top = chunks[1].y.saturating_add(1);
    let visible = (chunks[1].height.saturating_sub(2)) as usize;
    for (idx, _) in app.search.items.iter().enumerate().take(visible) {
        app.hit_areas.push((
            Rect::new(
                chunks[1].x.saturating_add(1),
                list_top.saturating_add(idx as u16),
                chunks[1].width.saturating_sub(2),
                1,
            ),
            MouseAction::SelectSearch(idx),
        ));
    }
}
