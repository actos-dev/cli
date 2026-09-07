use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::{App, CurrentTab};
use crate::tui::mouse::{self, MouseAction};
use crate::tui::views::actors::render_actors;
use crate::tui::views::detail::render_detail;
use crate::tui::views::feed::render_feed;
use crate::tui::views::help::{render_help, render_help_popup};
use crate::tui::views::inbox::render_inbox;
use crate::tui::views::profile::render_profile;
use crate::tui::views::saves::render_saves;
use crate::tui::views::search::render_search;
use crate::tui::views::tags::{render_tag_posts, render_tags};

/// Sekme tanımları: etiket + hedef ekran (sıralama çubuğu sırasıdır).
const TAB_DEFS: [(&str, CurrentTab); 8] = [
    (" Feed ", CurrentTab::Feed),
    (" Tags ", CurrentTab::Tags),
    (" Actors ", CurrentTab::Actors),
    (" Search ", CurrentTab::Search),
    (" Inbox ", CurrentTab::Inbox),
    (" Saves ", CurrentTab::Saves),
    (" Profile ", CurrentTab::Profile),
    (" Help ", CurrentTab::Help),
];

pub fn render_ui(frame: &mut Frame, app: &mut App) {
    app.hit_areas.clear();
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs header
            Constraint::Min(10),   // Main content
            Constraint::Length(2), // Status bar & hints
        ])
        .split(frame.area());

    // Top Tabs (manuel çizilir ki tıklama alanları bilinsin)
    let selected_tab = match app.current_tab {
        CurrentTab::Feed | CurrentTab::Detail => 0,
        CurrentTab::Tags | CurrentTab::TagPosts => 1,
        CurrentTab::Actors => 2,
        CurrentTab::Search => 3,
        CurrentTab::Inbox => 4,
        CurrentTab::Saves => 5,
        CurrentTab::Profile => 6,
        CurrentTab::Help => 7,
    };

    let bar = Block::default().title(" Actos TUI ").borders(Borders::ALL);
    let bar_inner = bar.inner(main_layout[0]);
    frame.render_widget(bar, main_layout[0]);

    let labels: Vec<&str> = TAB_DEFS.iter().map(|(label, _)| *label).collect();
    let areas = mouse::tab_bar_areas(&labels, bar_inner.x, bar_inner.y);
    for (i, ((label, tab), rect)) in TAB_DEFS.iter().zip(areas).enumerate() {
        app.hit_areas.push((rect, MouseAction::SwitchTab(*tab)));
        let style = if i == selected_tab {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(*label, style))),
            rect,
        );
    }

    // Main content
    match app.current_tab {
        CurrentTab::Feed => render_feed(frame, app, main_layout[1]),
        CurrentTab::Tags => render_tags(frame, app, main_layout[1]),
        CurrentTab::TagPosts => render_tag_posts(frame, app, main_layout[1]),
        CurrentTab::Actors => render_actors(frame, app, main_layout[1]),
        CurrentTab::Detail => render_detail(frame, app, main_layout[1]),
        CurrentTab::Search => render_search(frame, app, main_layout[1]),
        CurrentTab::Inbox => render_inbox(frame, app, main_layout[1]),
        CurrentTab::Saves => render_saves(frame, app, main_layout[1]),
        CurrentTab::Profile => render_profile(frame, app, main_layout[1]),
        CurrentTab::Help => render_help(frame, main_layout[1]),
    }

    // Status / Bottom Bar
    let status_line = Line::from(vec![
        Span::styled(
            format!(" Status: {} ", app.status_message),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | "),
        Span::styled(
            "q: Quit | Tab: Switch Tab | ?: Shortcuts | Enter: Open | Esc: Back | Click: Select/Open | Wheel: Scroll",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status_widget = Paragraph::new(status_line);
    frame.render_widget(status_widget, main_layout[2]);

    // Popup'lar: yardım ve giriş overlay'leri en üstte.
    if app.show_help_popup {
        render_help_popup(frame, frame.area());
    } else if let Some(overlay) = app.overlay.clone() {
        render_overlay(frame, frame.area(), &overlay);
    }
}

/// Ekranı ortalayan küçük kutu alanı hesaplar (yüzde cinsinden).
fn centered_rect(percent_x: u16, percent_y: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn render_overlay(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    overlay: &crate::tui::app::Overlay,
) {
    use crate::tui::app::Overlay as O;
    let popup = centered_rect(70, 40, area);
    let block = Block::default()
        .title(overlay.title())
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup);
    frame.render_widget(ratatui::widgets::Clear, popup);
    frame.render_widget(block, popup);
    match overlay {
        O::ConfirmDeletePost { post_id } => {
            let text = Paragraph::new(vec![
                Line::from(format!("Delete post {post_id}? This is permanent.")),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter: delete   Esc: cancel",
                    Style::default().fg(Color::Yellow),
                )),
            ]);
            frame.render_widget(text, inner);
        }
        _ if overlay.labeled_fields().is_some() => {
            let fields = overlay.labeled_fields().unwrap_or([
                ("", String::new()),
                ("", String::new()),
                ("", String::new()),
            ]);
            let focus_idx = match overlay.focus() {
                Some(crate::tui::app::OverlayField::First) => 0,
                Some(crate::tui::app::OverlayField::Second) => 1,
                _ => 2,
            };
            let mut lines = Vec::new();
            for (i, (label, value)) in fields.iter().enumerate() {
                if label.is_empty() {
                    continue;
                }
                let marker = if i == focus_idx { "> " } else { "  " };
                let style = if i == focus_idx {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let shown = if i == focus_idx {
                    format!("{value}█")
                } else {
                    value.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("{marker}{label}: "), style),
                    Span::raw(shown),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Tab: field   Enter: newline/submit   Ctrl+S: submit   Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )));
            let text = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
            frame.render_widget(text, inner);
        }
        _ => {
            let content = overlay.text().unwrap_or_default();
            let text = Paragraph::new(vec![
                Line::from(format!("{content}█")),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter: send   Esc: cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .wrap(ratatui::widgets::Wrap { trim: false });
            frame.render_widget(text, inner);
        }
    }
}
