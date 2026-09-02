use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::App;

pub fn render_profile(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(ref profile) = app.profile_info {
        let actor = &profile.actor;
        let stats = &profile.stats;

        let lines = vec![
            Line::from(vec![
                Span::styled("Username:     ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("@{}", actor.username),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Actor ID:     ", Style::default().fg(Color::Yellow)),
                Span::raw(&actor.id),
            ]),
            Line::from(vec![
                Span::styled("Actor Type:   ", Style::default().fg(Color::Yellow)),
                Span::raw(&actor.actor_type),
            ]),
            Line::from(vec![
                Span::styled("Display Name: ", Style::default().fg(Color::Yellow)),
                Span::raw(actor.display_name.as_deref().unwrap_or("(none)")),
            ]),
            Line::from(vec![
                Span::styled("Bio:          ", Style::default().fg(Color::Yellow)),
                Span::raw(actor.bio.as_deref().unwrap_or("(none)")),
            ]),
            Line::from(vec![
                Span::styled("Member Since: ", Style::default().fg(Color::Yellow)),
                Span::raw(&actor.created_at),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "─── Stats ─────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(vec![
                Span::styled("Total Posts:    ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", stats.post_count)),
            ]),
            Line::from(vec![
                Span::styled("Total Comments: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", stats.comment_count)),
            ]),
            Line::from(vec![
                Span::styled("Total Karma:    ", Style::default().fg(Color::Green)),
                Span::raw(format!("{}", stats.total_score)),
            ]),
        ];

        let widget = Paragraph::new(lines).block(
            Block::default()
                .title(" Actor Profile & Identity Card ")
                .borders(Borders::ALL),
        );

        frame.render_widget(widget, area);
    } else {
        let not_logged_in = Paragraph::new(vec![
            Line::from("Not authenticated or anonymous session."),
            Line::from(""),
            Line::from("Run 'actos auth login' to authenticate with an API key."),
        ])
        .block(Block::default().title(" Profile ").borders(Borders::ALL));

        frame.render_widget(not_logged_in, area);
    }
}
