use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::tui::app::{App, CurrentTab};
use crate::tui::views::detail::render_detail;
use crate::tui::views::feed::render_feed;
use crate::tui::views::help::{render_help, render_help_popup};
use crate::tui::views::profile::render_profile;
use crate::tui::views::search::render_search;

pub fn render_ui(frame: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs header
            Constraint::Min(10),   // Main content
            Constraint::Length(2), // Status bar & hints
        ])
        .split(frame.area());

    // Top Tabs
    let titles = vec![
        " [F1] Feed ",
        " [F2] Search ",
        " [F3] Profile ",
        " [F4] Help ",
    ];

    let selected_tab = match app.current_tab {
        CurrentTab::Feed | CurrentTab::Detail => 0,
        CurrentTab::Search => 1,
        CurrentTab::Profile => 2,
        CurrentTab::Help => 3,
    };

    let tabs = Tabs::new(titles)
        .select(selected_tab)
        .block(Block::default().title(" Actos TUI ").borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, main_layout[0]);

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
            "q: Quit | Tab: Switch Tab | ?: Shortcuts | Enter: Open | Esc: Back",
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
