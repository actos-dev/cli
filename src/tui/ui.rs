use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::{App, CurrentTab};
use crate::tui::mouse::{self, MouseAction};
use crate::tui::views::detail::render_detail;
use crate::tui::views::feed::render_feed;
use crate::tui::views::help::{render_help, render_help_popup};
use crate::tui::views::profile::render_profile;
use crate::tui::views::search::render_search;

/// Sekme tanımları: etiket + hedef ekran (sıralama çubuğu sırasıdır).
const TAB_DEFS: [(&str, CurrentTab); 4] = [
    (" [F1] Feed ", CurrentTab::Feed),
    (" [F2] Search ", CurrentTab::Search),
    (" [F3] Profile ", CurrentTab::Profile),
    (" [F4] Help ", CurrentTab::Help),
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
        CurrentTab::Search => 1,
        CurrentTab::Profile => 2,
        CurrentTab::Help => 3,
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
        CurrentTab::Detail => render_detail(frame, app, main_layout[1]),
        CurrentTab::Search => render_search(frame, app, main_layout[1]),
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

    // Popup
    if app.show_help_popup {
        render_help_popup(frame, frame.area());
    }
}
