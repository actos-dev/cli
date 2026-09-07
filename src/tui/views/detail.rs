use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::app::App;
use actos_sdk::actos_types::content::CommentNodeResponse;

fn build_comment_lines(node: &CommentNodeResponse, depth: usize, out: &mut Vec<Line>) {
    let indent = "  ".repeat(depth);
    let marker = if depth > 0 { "└─ " } else { "• " };

    let author_span = Span::styled(
        format!("@{}: ", node.content.author.username),
        Style::default().fg(Color::Cyan),
    );
    let body_span = Span::raw(node.content.body.clone());
    let score_span = Span::styled(
        format!(" (+{})", node.content.score),
        Style::default().fg(Color::DarkGray),
    );

    out.push(Line::from(vec![
        Span::raw(format!("{indent}{marker}")),
        author_span,
        body_span,
        score_span,
    ]));

    for reply in &node.replies {
        build_comment_lines(reply, depth + 1, out);
    }
}

pub fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);

    if let Some(ref post) = app.selected_post {
        let title_str = post.title.as_deref().unwrap_or("(no title)");
        let post_text = vec![
            Line::from(vec![
                Span::styled(
                    title_str,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ("),
                Span::styled(
                    format!("@{}", post.author.username),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(")"),
            ]),
            Line::from(vec![
                Span::raw("Score: "),
                Span::styled(format!("{}", post.score), Style::default().fg(Color::Green)),
                Span::raw(format!(" | Created: {}", post.created_at)),
            ]),
            Line::from(""),
            Line::from(crate::tui::app::strip_leading_title(
                &post.body,
                post.title.as_deref(),
            )),
        ];

        let post_widget = Paragraph::new(post_text)
            .block(
                Block::default()
                    .title(" Post Details ")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(post_widget, chunks[0]);

        let mut comment_lines = Vec::new();
        if app.post_comments.is_empty() {
            comment_lines.push(Line::from("No comments yet."));
        } else {
            for comment in &app.post_comments {
                build_comment_lines(comment, 0, &mut comment_lines);
            }
        }

        let comments_widget = Paragraph::new(comment_lines)
            .block(Block::default().title(" Comments ").borders(Borders::ALL))
            .wrap(Wrap { trim: false });

        frame.render_widget(comments_widget, chunks[1]);
    } else {
        let empty_widget = Block::default()
            .title(" No Post Selected ")
            .borders(Borders::ALL);
        frame.render_widget(empty_widget, area);
    }
}
