use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

fn kv(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{key:<22}"),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(desc.to_string()),
    ])
}

pub fn render_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "Actos TUI Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "— Navigation —",
            Style::default().fg(Color::Green),
        )),
        kv("Tab / Shift+Tab", "Next / previous tab"),
        kv("F1..F8", "Jump to Feed Tags Actors Search Inbox Saves Profile Help"),
        kv("j / k, Up / Down", "Move selection (scroll in Detail)"),
        kv("PgUp / PgDn, Home / End", "Scroll detail (Detail only)"),
        kv("Enter", "Open post / run search / activate"),
        kv("o / Right", "Older page (Feed Tags Inbox Saves Profile)"),
        kv("Esc", "Back / close popup"),
        kv("Click, Wheel", "Select/open tabs and rows, scroll"),
        Line::from(""),
        Line::from(Span::styled("— Feed —", Style::default().fg(Color::Green))),
        kv("s / w / a / f", "Sort / window / actor filter / following toggle"),
        kv("r", "Refresh"),
        kv("c", "Compose new post"),
        Line::from(""),
        Line::from(Span::styled(
            "— Detail —",
            Style::default().fg(Color::Green),
        )),
        kv("+ / - / 0", "Upvote / downvote / retract"),
        kv("S / X", "Save / unsave"),
        kv("r / R", "Reply / report"),
        kv("[ / ]", "Select comment"),
        kv("e / E", "Edit selected comment / edit post"),
        kv("D", "Delete post (asks first)"),
        Line::from(""),
        Line::from(Span::styled("— Other —", Style::default().fg(Color::Green))),
        kv("Ctrl+T (Search)", "Cycle post / comment / actor"),
        kv("t (Tags→Actors→Profile)", "Tag sort / actor filter / posts-comments"),
        kv("u / R / A (Inbox)", "Unread filter / mark read / mark all read"),
        kv("f / u (Profile)", "Follow / unfollow"),
        kv("Tab (composer)", "Next field"),
        kv("Ctrl+S (composer)", "Publish / save"),
        kv("q", "Quit (never inside a text box)"),
        kv("?", "This popup"),
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
        Line::from("  q / Esc      - Quit / back"),
        Line::from("  Tab, F1-F8   - Switch tabs"),
        Line::from("  j / k        - Move (scroll in Detail)"),
        Line::from("  Enter / o    - Open / older page"),
        Line::from("  Click/Wheel  - Mouse select, open, scroll"),
        Line::from("  ?            - Close this popup"),
        Line::from(""),
        Line::from(Span::styled(
            "Full map on the Help tab.",
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
