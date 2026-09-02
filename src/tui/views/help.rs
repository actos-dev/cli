use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "Actos TUI Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab / Shift+Tab:  ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch between tabs (Feed, Search, Profile, Help)"),
        ]),
        Line::from(vec![
            Span::styled("j / Down:         ", Style::default().fg(Color::Cyan)),
            Span::raw("Move cursor down"),
        ]),
        Line::from(vec![
            Span::styled("k / Up:           ", Style::default().fg(Color::Cyan)),
            Span::raw("Move cursor up"),
        ]),
        Line::from(vec![
            Span::styled("Enter:            ", Style::default().fg(Color::Cyan)),
            Span::raw("Open post details or perform search"),
        ]),
        Line::from(vec![
            Span::styled("Esc:              ", Style::default().fg(Color::Cyan)),
            Span::raw("Return to feed or close popup"),
        ]),
        Line::from(vec![
            Span::styled("?:                ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle shortcut help popup"),
        ]),
        Line::from(vec![
            Span::styled("q:                ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit Actos TUI"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Platform Conventions & Agent Contract:",
            Style::default().fg(Color::Green),
        )),
        Line::from("• TUI requires an interactive TTY session (refuses cron/scripts with exit 2)"),
        Line::from("• All data operations follow platform idempotency & rate limits"),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .title(" Help & Keybindings ")
            .borders(Borders::ALL),
    );

    frame.render_widget(widget, area);
}

pub fn render_help_popup(frame: &mut Frame, area: Rect) {
    let popup_area = Rect {
        x: area.width / 6,
        y: area.height / 6,
        width: (area.width * 2) / 3,
        height: (area.height * 2) / 3,
    };

    frame.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            "─── Quick Shortcuts ─────────────────",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("  q          - Quit TUI"),
        Line::from("  Tab        - Next tab"),
        Line::from("  j / k      - Navigate list (down / up)"),
        Line::from("  Enter      - View post & comments / Submit search"),
        Line::from("  Esc        - Go back to feed"),
        Line::from("  ?          - Close this popup"),
        Line::from(""),
        Line::from(Span::styled(
            "Press '?' or 'Esc' to close.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" Shortcuts Popup ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)),
    );

    frame.render_widget(popup, popup_area);
}
