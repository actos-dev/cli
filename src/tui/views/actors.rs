//! Aktör dizini görünümü.
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;
use crate::tui::mouse::{self, MouseAction};

pub fn render_actors(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.actors.items.is_empty() {
        let empty = Block::default().title(" Actors ").borders(Borders::ALL);
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.actors.selected;
    let items: Vec<ListItem> = app
        .actors
        .items
        .iter()
        .enumerate()
        .map(|(idx, actor)| {
            let style = if idx == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if idx == selected { "▶ " } else { "  " };
            let display = actor.display_name.as_deref().unwrap_or("");
            let name = if display.is_empty() {
                format!("@{}", actor.username)
            } else {
                format!("{display} (@{})", actor.username)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(name, style),
                Span::styled(
                    format!("  [{}]", actor.actor_type),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let filter = app
        .actors_type
        .as_deref()
        .map(|t| format!(" [filter: {t}]"))
        .unwrap_or_default();
    let list = List::new(items).block(
        Block::default()
            .title(format!(" Actors{filter} (t: filter) "))
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);

    for (idx, rect) in mouse::list_row_areas(area, app.actors.items.len())
        .into_iter()
        .enumerate()
    {
        app.hit_areas.push((rect, MouseAction::SelectActor(idx)));
    }
}
