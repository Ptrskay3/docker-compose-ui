use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListDirection, ListItem, Paragraph},
    Frame,
};

// const CONSTRAINT_50_50: [Constraint; 2] = [Constraint::Percentage(70), Constraint::Percentage(30)];
use crate::app::App;

fn create_legend<'a>() -> Paragraph<'a> {
    let text = Line::default().spans(vec![
        Span::styled(
            "(a)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" start all containers, "),
        Span::styled(
            "(Enter)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" to start selected, "),
        Span::styled(
            "(s)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" to stop selected."),
        Span::styled(
            "(q)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" to quit."),
    ]);

    Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Keys"))
}

pub fn render(app: &mut App, frame: &mut Frame) {
    let items: Vec<ListItem> = app
        .compose_content
        .compose
        .services
        .0
        .keys()
        .enumerate()
        .map(|(i, s)| {
            let content = Text::raw(s);
            let style = if app.running_container_names.iter().any(|m| m.contains(s)) {
                // Apply red style to the first item
                Style::default().fg(Color::LightGreen)
            } else if app.compose_content.queued.contains(&i) {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::ITALIC)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom)
        .block(
            Block::bordered()
                .title("Docker Compose UI")
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
        );

    frame.render_stateful_widget(list, frame.area(), &mut app.compose_content.state);
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(size);

    let legend = create_legend();
    frame.render_widget(legend, chunks[1]);
}
