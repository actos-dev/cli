//! Tags listesi ve tag postları görünümleri.
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;
use crate::tui::mouse::{self, MouseAction};
use crate::tui::views::row_title;

pub fn render_tags(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.tags.items.is_empty() {
        let empty = Block::default().title(" Tags ").borders(Borders::ALL);
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.tags.selected;
    let items: Vec<ListItem> = app
        .tags
        .items
        .iter()
        .enumerate()
        .map(|(idx, tag)| {
            let style = if idx == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if idx == selected { "▶ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(format!("#{}", tag.name), style),
                Span::styled(
                    format!("  ({} posts)", tag.post_count),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Tags (Enter: open posts) ")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);

    for (idx, rect) in mouse::list_row_areas(area, app.tags.items.len())
        .into_iter()
        .enumerate()
    {
        app.hit_areas.push((rect, MouseAction::SelectTag(idx)));
    }
}

pub fn render_tag_posts(frame: &mut Frame, app: &mut App, area: Rect) {
    let selected = app.tag_posts.selected;
    let items: Vec<ListItem> = app
        .tag_posts
        .items
        .iter()
        .enumerate()
        .map(|(idx, post)| {
            let style = if idx == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if idx == selected { "▶ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(row_title(post), style),
                Span::raw("  by "),
                Span::styled(
                    format!("@{}", post.author.username),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("  [Score: {}]", post.score),
                    Style::default().fg(Color::Green),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(
                " #{} [{}] (Enter: open, s: sort) ",
                app.tag_posts_name, app.tag_posts_sort
            ))
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);

    for (idx, rect) in mouse::list_row_areas(area, app.tag_posts.items.len())
        .into_iter()
        .enumerate()
    {
        app.hit_areas.push((rect, MouseAction::SelectTagPost(idx)));
    }
}
