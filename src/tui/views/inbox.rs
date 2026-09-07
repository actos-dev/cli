//! Inbox (bildirimler) görünümü.
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::App;
use crate::tui::mouse::{self, MouseAction};

pub fn render_inbox(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.inbox.items.is_empty() {
        let empty = Block::default()
            .title(" Inbox ")
            .borders(Borders::ALL);
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.inbox.selected;
    let items: Vec<ListItem> = app
        .inbox
        .items
        .iter()
        .enumerate()
        .map(|(idx, notif)| {
            let unread = notif.read_at.is_none();
            let style = if idx == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if unread {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let prefix = if idx == selected {
                "▶ "
            } else if unread {
                "● "
            } else {
                "  "
            };
            let who = notif
                .actor
                .as_ref()
                .map(|a| format!("@{}", a.username))
                .unwrap_or_else(|| "system".to_string());
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(format!("{} {}", who, notif.kind), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(
                " Inbox ({} unread{}) (Enter: open, R: read, A: all read, u: unread) ",
                app.inbox_unread_total,
                if app.inbox_unread_only {
                    ", unread only"
                } else {
                    ""
                }
            ))
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);

    for (idx, rect) in mouse::list_row_areas(area, app.inbox.items.len())
        .into_iter()
        .enumerate()
    {
        app.hit_areas.push((rect, MouseAction::SelectNotif(idx)));
    }
}
