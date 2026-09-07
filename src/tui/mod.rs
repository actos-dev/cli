#[cfg(feature = "tui")]
pub mod app;
#[cfg(feature = "tui")]
pub mod mouse;
#[cfg(feature = "tui")]
pub mod ui;
#[cfg(feature = "tui")]
pub mod views;

use crate::client::ApiClient;
use crate::error::CliError;

#[cfg(feature = "tui")]
pub async fn run_tui(client: &ApiClient) -> Result<(), CliError> {
    use std::io::IsTerminal;

    // Ajan Sözleşmesi §2 kural 5
    if !std::io::stdout().is_terminal() {
        return Err(CliError::Usage(
            "actos tui requires an interactive terminal (TTY)".to_string(),
        ));
    }

    use crossterm::{
        execute,
        event::{DisableMouseCapture, EnableMouseCapture},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    enable_raw_mode().map_err(|e| CliError::Io(format!("Failed to enable raw mode: {e}")))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| CliError::Io(format!("Failed to enter alternate screen: {e}")))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .map_err(|e| CliError::Io(format!("Failed to initialize terminal: {e}")))?;

    let mut app = app::App::new(client).await;

    let res = run_loop(&mut terminal, &mut app, client).await;

    // Terminal durumunu geri yükle
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    res
}

#[cfg(feature = "tui")]
async fn handle_mouse(
    app: &mut app::App,
    client: &ApiClient,
    m: crossterm::event::MouseEvent,
) {
    use crossterm::event::{MouseButton, MouseEventKind};

    match m.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if app.show_help_popup {
                app.show_help_popup = false;
                return;
            }
            match mouse::hit_test(&app.hit_areas, m.column, m.row) {
                Some(mouse::MouseAction::SwitchTab(tab)) => {
                    if tab != app::CurrentTab::Detail {
                        app.current_tab = tab;
                    }
                }
                Some(mouse::MouseAction::SelectFeed(i)) => {
                    if app.feed.selected == i {
                        app.open_selected_post(client).await;
                    } else {
                        app.feed.selected = i;
                    }
                }
                Some(mouse::MouseAction::SelectSearch(i)) => {
                    app.search.selected = i;
                }
                None => {}
            }
        }
        MouseEventKind::ScrollUp => app.move_up(),
        MouseEventKind::ScrollDown => app.move_down(),
        _ => {}
    }
}

#[cfg(feature = "tui")]
async fn run_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
    client: &ApiClient,
) -> Result<(), CliError> {
    use crossterm::event::{self, Event, KeyCode};
    use std::time::Duration;

    while !app.should_quit {
        terminal
            .draw(|f| ui::render_ui(f, app))
            .map_err(|e| CliError::Io(format!("Draw error: {e}")))?;

        if event::poll(Duration::from_millis(100))
            .map_err(|e| CliError::Io(format!("Event poll error: {e}")))?
        {
            match event::read().map_err(|e| CliError::Io(format!("Event read error: {e}")))? {
                Event::Mouse(m) => handle_mouse(app, client, m).await,
                Event::Key(key) => {
            if app.show_help_popup {
                if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
                    app.show_help_popup = false;
                }
                continue;
            }

            match key.code {
                // B4: arama kutusunda `q` harftir, çıkış değil.
                KeyCode::Char('q') if app.current_tab != app::CurrentTab::Search => {
                    app.should_quit = true;
                }
                KeyCode::Char('q') => {
                    app.search_query.push('q');
                }
                KeyCode::Char('?') => {
                    app.show_help_popup = !app.show_help_popup;
                }
                KeyCode::Tab => {
                    app.next_tab();
                }
                KeyCode::BackTab => {
                    app.previous_tab();
                }
                KeyCode::Esc => {
                    app.go_back();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    app.move_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.move_up();
                }
                // B3: sekme başlıklarında yazan F1-F4 gerçekten çalışır.
                KeyCode::F(1) => {
                    app.current_tab = app::CurrentTab::Feed;
                }
                KeyCode::F(2) => {
                    app.current_tab = app::CurrentTab::Search;
                }
                KeyCode::F(3) => {
                    app.current_tab = app::CurrentTab::Profile;
                }
                KeyCode::F(4) => {
                    app.current_tab = app::CurrentTab::Help;
                }
                KeyCode::Char('t')
                    if app.current_tab == app::CurrentTab::Search
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    app.cycle_search_type();
                }
                KeyCode::Enter => match app.current_tab {
                    app::CurrentTab::Feed => {
                        app.open_selected_post(client).await;
                    }
                    app::CurrentTab::Search => {
                        app.perform_search(client).await;
                    }
                    _ => {}
                },
                KeyCode::Backspace if app.current_tab == app::CurrentTab::Search => {
                    app.search_query.pop();
                }
                KeyCode::Char(c) if app.current_tab == app::CurrentTab::Search => {
                    app.search_query.push(c);
                }
                _ => {}
            }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "tui"))]
pub async fn run_tui(_client: &ApiClient) -> Result<(), CliError> {
    Err(CliError::Usage(
        "actos was compiled without TUI support. Recompile with '--features tui'".to_string(),
    ))
}
