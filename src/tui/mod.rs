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
                        enter_tab(app, client, tab).await;
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
                Some(mouse::MouseAction::SelectTag(i)) => {
                    if app.tags.selected == i {
                        app.open_tag_posts(client).await;
                    } else {
                        app.tags.selected = i;
                    }
                }
                Some(mouse::MouseAction::SelectTagPost(i)) => {
                    if app.tag_posts.selected == i {
                        app.open_selected_tag_post(client).await;
                    } else {
                        app.tag_posts.selected = i;
                    }
                }
                Some(mouse::MouseAction::SelectActor(i)) => {
                    if app.actors.selected == i {
                        if let Some(actor) = app.actors.selected_item() {
                            let username = actor.username.clone();
                            app.open_actor_profile(client, &username).await;
                        }
                    } else {
                        app.actors.selected = i;
                    }
                }
                Some(mouse::MouseAction::SelectNotif(i)) => {
                    app.inbox.selected = i;
                }
                Some(mouse::MouseAction::SelectSave(i)) => {
                    if app.saves.selected == i {
                        app.open_selected_save(client).await;
                    } else {
                        app.saves.selected = i;
                    }
                }
                Some(mouse::MouseAction::SelectSearchActor(i)) => {
                    if app.search_actors.selected == i {
                        app.open_selected_search(client).await;
                    } else {
                        app.search_actors.selected = i;
                    }
                }
                Some(mouse::MouseAction::SelectProfilePost(i)) => {
                    if app.profile_posts.selected == i {
                        app.open_selected_profile_post(client).await;
                    } else {
                        app.profile_posts.selected = i;
                    }
                }
                None => {}
            }
        }
        MouseEventKind::ScrollUp => {
            if app.current_tab == app::CurrentTab::Detail {
                app.scroll_detail_by(-3);
            } else {
                app.move_up();
            }
        }
        MouseEventKind::ScrollDown => {
            if app.current_tab == app::CurrentTab::Detail {
                app.scroll_detail_by(3);
            } else {
                app.move_down();
            }
        }
        _ => {}
    }
}

/// Sekmeye geç + ilk girişte tembel yükle (boşsa).
#[cfg(feature = "tui")]
async fn enter_tab(app: &mut app::App, client: &ApiClient, tab: app::CurrentTab) {
    app.current_tab = tab;
    match tab {
        app::CurrentTab::Tags if app.tags.items.is_empty() => {
            app.refresh_tags(client).await;
        }
        app::CurrentTab::Actors if app.actors.items.is_empty() => {
            app.refresh_actors(client).await;
        }
        app::CurrentTab::Inbox if app.inbox.items.is_empty() => {
            app.refresh_inbox(client).await;
        }
        app::CurrentTab::Saves if app.saves.items.is_empty() => {
            app.refresh_saves(client).await;
        }
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

            // Overlay açıkken tuşlar giriş kutusuna gider.
            if app.overlay.is_some() {
                match key.code {
                    KeyCode::Esc => {
                        app.overlay = None;
                    }
                    KeyCode::Tab => {
                        if let Some(o) = app.overlay.as_mut() {
                            o.cycle_focus();
                        }
                    }
                    KeyCode::Enter => {
                        let newline = matches!(
                            app.overlay.as_ref().map(|o| o.enter_action()),
                            Some(crate::tui::app::EnterAction::Newline)
                        );
                        if newline {
                            if let Some(o) = app.overlay.as_mut() {
                                o.push_char('\n');
                            }
                        } else {
                            app.submit_overlay(client).await;
                        }
                    }
                    KeyCode::Char('s')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        app.submit_overlay(client).await;
                    }
                    KeyCode::Backspace => {
                        if let Some(o) = app.overlay.as_mut() {
                            o.pop_char();
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(o) = app.overlay.as_mut() {
                            o.push_char(c);
                        }
                    }
                    _ => {}
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
                    if app.current_tab == app::CurrentTab::Detail {
                        app.scroll_detail_by(1);
                    } else {
                        app.move_down();
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if app.current_tab == app::CurrentTab::Detail {
                        app.scroll_detail_by(-1);
                    } else {
                        app.move_up();
                    }
                }
                KeyCode::PageDown if app.current_tab == app::CurrentTab::Detail => {
                    app.scroll_detail_by(10);
                }
                KeyCode::PageUp if app.current_tab == app::CurrentTab::Detail => {
                    app.scroll_detail_by(-10);
                }
                KeyCode::Home if app.current_tab == app::CurrentTab::Detail => {
                    app.detail_scroll = 0;
                }
                KeyCode::End if app.current_tab == app::CurrentTab::Detail => {
                    app.detail_scroll = app.detail_lines;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    if app.current_tab == app::CurrentTab::Detail {
                        app.vote_current(client, 1).await;
                    }
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    if app.current_tab == app::CurrentTab::Detail {
                        app.vote_current(client, -1).await;
                    }
                }
                KeyCode::Char('0') if app.current_tab == app::CurrentTab::Detail => {
                    app.vote_current(client, 0).await;
                }
                KeyCode::Char('S') if app.current_tab == app::CurrentTab::Detail => {
                    app.save_current(client, true).await;
                }
                KeyCode::Char('X') if app.current_tab == app::CurrentTab::Detail => {
                    app.save_current(client, false).await;
                }
                KeyCode::Char('r') if app.current_tab == app::CurrentTab::Detail => {
                    app.open_overlay_reply(client);
                }
                KeyCode::Char('R') if app.current_tab == app::CurrentTab::Detail => {
                    app.open_overlay_report(client);
                }
                KeyCode::Char('D') if app.current_tab == app::CurrentTab::Detail => {
                    app.open_confirm_delete(client);
                }
                KeyCode::Char('E') if app.current_tab == app::CurrentTab::Detail => {
                    app.open_edit_post(client);
                }
                KeyCode::Char('e') if app.current_tab == app::CurrentTab::Detail => {
                    app.open_edit_comment(client);
                }
                KeyCode::Char('[') if app.current_tab == app::CurrentTab::Detail => {
                    app.cycle_comment(-1);
                }
                KeyCode::Char(']') if app.current_tab == app::CurrentTab::Detail => {
                    app.cycle_comment(1);
                }
                KeyCode::Char('c') if app.current_tab == app::CurrentTab::Feed => {
                    app.open_composer(client);
                }
                KeyCode::Char('E') if app.current_tab == app::CurrentTab::Profile => {
                    app.open_edit_profile(client);
                }
                KeyCode::Char('r') if app.current_tab == app::CurrentTab::Feed => {
                    app.refresh_feed(client).await;
                }
                KeyCode::Char('s') if app.current_tab == app::CurrentTab::Feed => {
                    app.cycle_feed_sort();
                    app.refresh_feed(client).await;
                }
                KeyCode::Char('w') if app.current_tab == app::CurrentTab::Feed => {
                    app.cycle_feed_window();
                    app.refresh_feed(client).await;
                }
                KeyCode::Char('a') if app.current_tab == app::CurrentTab::Feed => {
                    app.cycle_feed_actor();
                    app.refresh_feed(client).await;
                }
                KeyCode::Char('f') if app.current_tab == app::CurrentTab::Feed => {
                    app.toggle_feed_following();
                    app.refresh_feed(client).await;
                }
                KeyCode::Char('t') if app.current_tab == app::CurrentTab::Actors => {
                    app.cycle_actors_type();
                    app.refresh_actors(client).await;
                }
                KeyCode::Char('s') if app.current_tab == app::CurrentTab::TagPosts => {
                    app.cycle_tag_posts_sort();
                    app.refresh_tag_posts(client).await;
                }
                KeyCode::Char('o') | KeyCode::Right
                    if app.current_tab == app::CurrentTab::TagPosts =>
                {
                    app.load_tag_posts_older(client).await;
                }
                KeyCode::Char('u') if app.current_tab == app::CurrentTab::Inbox => {
                    app.toggle_inbox_unread();
                    app.refresh_inbox(client).await;
                }
                KeyCode::Char('R') if app.current_tab == app::CurrentTab::Inbox => {
                    app.mark_selected_read(client).await;
                }
                KeyCode::Char('A') if app.current_tab == app::CurrentTab::Inbox => {
                    app.mark_all_read(client).await;
                }
                KeyCode::Char('t') if app.current_tab == app::CurrentTab::Profile => {
                    app.cycle_profile_tab();
                    app.refresh_profile_content(client).await;
                }
                KeyCode::Char('f') if app.current_tab == app::CurrentTab::Profile => {
                    app.follow_profile(client, true).await;
                }
                KeyCode::Char('u') if app.current_tab == app::CurrentTab::Profile => {
                    app.follow_profile(client, false).await;
                }
                // B3: sekme başlıklarında yazan F1-F8 gerçekten çalışır.
                KeyCode::F(1) => {
                    enter_tab(app, client, app::CurrentTab::Feed).await;
                }
                KeyCode::F(2) => {
                    enter_tab(app, client, app::CurrentTab::Search).await;
                }
                KeyCode::F(3) => {
                    enter_tab(app, client, app::CurrentTab::Profile).await;
                }
                KeyCode::F(4) => {
                    enter_tab(app, client, app::CurrentTab::Help).await;
                }
                KeyCode::F(5) => {
                    enter_tab(app, client, app::CurrentTab::Tags).await;
                }
                KeyCode::F(6) => {
                    enter_tab(app, client, app::CurrentTab::Actors).await;
                }
                KeyCode::F(7) => {
                    enter_tab(app, client, app::CurrentTab::Inbox).await;
                }
                KeyCode::F(8) => {
                    enter_tab(app, client, app::CurrentTab::Saves).await;
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
                    app::CurrentTab::Tags => {
                        app.open_tag_posts(client).await;
                    }
                    app::CurrentTab::TagPosts => {
                        app.open_selected_tag_post(client).await;
                    }
                    app::CurrentTab::Actors => {
                        if let Some(actor) = app.actors.selected_item() {
                            let username = actor.username.clone();
                            app.open_actor_profile(client, &username).await;
                        }
                    }
                    app::CurrentTab::Search => {
                        app.perform_search(client).await;
                    }
                    app::CurrentTab::Inbox => {
                        app.open_selected_notification(client).await;
                    }
                    app::CurrentTab::Saves => {
                        app.open_selected_save(client).await;
                    }
                    app::CurrentTab::Profile => {
                        app.open_selected_profile_post(client).await;
                    }
                    _ => {}
                },
                KeyCode::Char('o') | KeyCode::Right => match app.current_tab {
                    app::CurrentTab::Feed => {
                        app.load_older(client).await;
                    }
                    app::CurrentTab::Tags => {
                        app.load_tags_older(client).await;
                    }
                    app::CurrentTab::TagPosts => {
                        app.load_tag_posts_older(client).await;
                    }
                    app::CurrentTab::Inbox => {
                        app.load_inbox_older(client).await;
                    }
                    app::CurrentTab::Saves => {
                        app.load_saves_older(client).await;
                    }
                    app::CurrentTab::Profile => {
                        app.load_profile_older(client).await;
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
