//! Kaydedilenler görünümü.
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;
use crate::tui::mouse::{self, MouseAction};
use crate::tui::views::row_title;

pub fn render_saves(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.saves.items.is_empty() {
        let empty = Block::default()
            .title(" Saves ")
            .borders(Borders::ALL);
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.saves.selected;
    let items: Vec<ListItem> = app
        .saves
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
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
                Span::styled(row_title(item), style),
                Span::raw("  by "),
                Span::styled(
                    format!("@{}", item.author.username),
                    Style::default().fg(Color::Cyan),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Saves (Enter: open) ")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);

    for (idx, rect) in mouse::list_row_areas(area, app.saves.items.len())
        .into_iter()
        .enumerate()
    {
        app.hit_areas.push((rect, MouseAction::SelectSave(idx)));
    }
}
