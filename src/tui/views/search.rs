use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::tui::app::App;
use crate::tui::mouse::MouseAction;
use crate::tui::views::row_title;

pub fn render_search(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
        .split(area);

    let input_line = Line::from(vec![
        Span::styled("Query: ", Style::default().fg(Color::Yellow)),
        Span::raw(&app.search_query),
        Span::styled("█", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("  [{}] (Ctrl+T: type)", app.search_type),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let input_widget = Paragraph::new(input_line).block(
        Block::default()
            .title(" Search Posts (Type and press Enter) ")
            .borders(Borders::ALL),
    );

    frame.render_widget(input_widget, chunks[0]);

    if app.search_type == "actor" {
        return render_search_actors(frame, app, chunks[1]);
    }

    if app.search.items.is_empty() {
        let empty = Paragraph::new("No search results to display.")
            .block(Block::default().title(" Results ").borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let selected = app.search.selected;
    let items: Vec<ListItem> = app
        .search
        .items
        .iter()
        .enumerate()
        .map(|(idx, post)| {
            let is_selected = idx == selected;
            let prefix = if is_selected { "▶ " } else { "  " };

            let title_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let title_str = row_title(post);
            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(format!("{:<40}", title_str), title_style),
                Span::raw("  by "),
                Span::styled(
                    format!("@{}", post.author.username),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!("  [Score: {}]", post.score)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Search Results ")
            .borders(Borders::ALL),
    );

    frame.render_widget(list, chunks[1]);

    let list_top = chunks[1].y.saturating_add(1);
    let visible = (chunks[1].height.saturating_sub(2)) as usize;
    for (idx, _) in app.search.items.iter().enumerate().take(visible) {
        app.hit_areas.push((
            Rect::new(
                chunks[1].x.saturating_add(1),
                list_top.saturating_add(idx as u16),
                chunks[1].width.saturating_sub(2),
                1,
            ),
            MouseAction::SelectSearch(idx),
        ));
    }
}

fn render_search_actors(frame: &mut Frame, app: &mut App, area: Rect) {
    use crate::tui::mouse::MouseAction as MA;

    if app.search_actors.items.is_empty() {
        let empty = Paragraph::new("No actors found.")
            .block(Block::default().title(" Actors ").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.search_actors.selected;
    let items: Vec<ListItem> = app
        .search_actors
        .items
        .iter()
        .enumerate()
        .map(|(idx, actor)| {
            let style = if idx == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if idx == selected { "▶ " } else { "  " };
            let display = actor.display_name.as_deref().unwrap_or("");
            let name = if display.is_empty() {
                format!("@{}", actor.username)
            } else {
                format!("{display} (@{})", actor.username)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(name, style),
                Span::styled(
                    format!("  [{}]", actor.actor_type),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Actors (Enter: open profile) ")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);

    let top = area.y.saturating_add(1);
    let visible = (area.height.saturating_sub(2)) as usize;
    for (idx, _) in app.search_actors.items.iter().enumerate().take(visible) {
        app.hit_areas.push((
            Rect::new(
                area.x.saturating_add(1),
                top.saturating_add(idx as u16),
                area.width.saturating_sub(2),
                1,
            ),
            MA::SelectSearchActor(idx),
        ));
    }
}
