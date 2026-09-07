//! Profil görünümü: başlık kartı + Posts/Comments sekmeleri.
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::tui::app::App;
use crate::tui::mouse::{self, MouseAction};
use crate::tui::views::row_title;

pub fn render_profile(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(profile) = app.profile_info.clone() else {
        let not_logged_in = Paragraph::new(vec![
            Line::from("Not authenticated or anonymous session."),
            Line::from(""),
            Line::from("Run 'actos auth login' to authenticate with an API key."),
        ])
        .block(Block::default().title(" Profile ").borders(Borders::ALL));
        frame.render_widget(not_logged_in, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(1),
            Constraint::Min(5),
        ])
        .split(area);

    let actor = &profile.actor;
    let stats = &profile.stats;
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                actor.display_name.as_deref().unwrap_or(&actor.username),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  @{}  [{}]", actor.username, actor.actor_type),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(Span::raw(actor.bio.as_deref().unwrap_or("(no bio)"))),
        Line::from(Span::styled(
            format!(
                "{} posts · {} comments · score {} · trust {}",
                stats.post_count, stats.comment_count, stats.total_score, actor.trust_level
            ),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "f: follow · u: unfollow",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().title(" Profile ").borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let tabs = if app.profile_posts_tab {
        Line::from(vec![
            Span::styled(
                "[Posts]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  Comments"),
        ])
    } else {
        Line::from(vec![
            Span::raw("Posts  "),
            Span::styled(
                "[Comments]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
    frame.render_widget(Paragraph::new(tabs), chunks[1]);

    if app.profile_posts.items.is_empty() {
        let empty = Paragraph::new("Nothing here yet.").block(
            Block::default()
                .title(if app.profile_posts_tab {
                    " Posts "
                } else {
                    " Comments "
                })
                .borders(Borders::ALL),
        );
        frame.render_widget(empty, chunks[2]);
        return;
    }

    let selected = app.profile_posts.selected;
    let items: Vec<ListItem> = app
        .profile_posts
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
                Span::styled(
                    format!("  [Score: {}]", item.score),
                    Style::default().fg(Color::Green),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" (Enter: open, t: posts/comments) ")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, chunks[2]);

    for (idx, rect) in mouse::list_row_areas(chunks[2], app.profile_posts.items.len())
        .into_iter()
        .enumerate()
    {
        app.hit_areas
            .push((rect, MouseAction::SelectProfilePost(idx)));
    }
}
