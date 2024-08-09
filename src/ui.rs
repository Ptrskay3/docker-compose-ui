use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListDirection, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, DockerModifier};

fn create_legend<'a>() -> Paragraph<'a> {
    let text = Line::default().spans(vec![
        Span::styled(
            "(Enter)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" start selected, "),
        Span::styled(
            "(a)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" start all containers, "),
        Span::styled(
            "(s)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" stop selected, "),
        Span::styled(
            "(x)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" stop all containers, "),
        Span::styled(
            "(r)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" restart container, "),
        Span::styled(
            "(q)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" to quit."),
    ]);

    Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Keys")
            .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
    )
}

fn create_docker_modifiers(modifiers: DockerModifier) -> Paragraph<'static> {
    let style_on = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Green);

    let style_off = Style::default().fg(Color::Red);
    let text = Line::default().spans(vec![
        Span::raw("(1) Build: "),
        Span::styled(
            if modifiers.contains(DockerModifier::BUILD) {
                "ON"
            } else {
                "OFF"
            },
            if modifiers.contains(DockerModifier::BUILD) {
                style_on
            } else {
                style_off
            },
        ),
        Span::raw(", (2) Force recreate: "),
        Span::styled(
            if modifiers.contains(DockerModifier::FORCE_RECREATE) {
                "ON"
            } else {
                "OFF"
            },
            if modifiers.contains(DockerModifier::FORCE_RECREATE) {
                style_on
            } else {
                style_off
            },
        ),
        Span::raw(", (3) Pull always: "),
        Span::styled(
            if modifiers.contains(DockerModifier::PULL_ALWAYS) {
                "ON"
            } else {
                "OFF"
            },
            if modifiers.contains(DockerModifier::PULL_ALWAYS) {
                style_on
            } else {
                style_off
            },
        ),
        Span::raw(", (4) Abort on container failure: "),
        Span::styled(
            if modifiers.contains(DockerModifier::ABORT_ON_CONTAINER_FAILURE) {
                "ON"
            } else {
                "OFF"
            },
            if modifiers.contains(DockerModifier::ABORT_ON_CONTAINER_FAILURE) {
                style_on
            } else {
                style_off
            },
        ),
        Span::raw(", (5) No deps: "),
        Span::styled(
            if modifiers.contains(DockerModifier::NO_DEPS) {
                "ON"
            } else {
                "OFF"
            },
            if modifiers.contains(DockerModifier::NO_DEPS) {
                style_on
            } else {
                style_off
            },
        ),
    ]);

    Paragraph::new(text).block(
        Block::default()
            .title("Docker Modifiers")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
    )
}

pub fn render(app: &mut App, frame: &mut Frame) {
    let size = frame.area();
    let main_and_legend = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(size);

    let main_and_modifier = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(main_and_legend[0]);

    let main_and_logs = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_and_modifier[0]);

    frame.render_widget(
        Paragraph::new(
            app.compose_content
                .log_area_content
                .as_deref()
                .unwrap_or_default(),
        )
        .block(
            Block::bordered()
                .title("Log area")
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
        ),
        main_and_logs[1],
    );

    let items: Vec<ListItem> = app
        .compose_content
        .compose
        .services
        .0
        .keys()
        .enumerate()
        .map(|(i, s)| {
            let content = Text::raw(s);
            let style = if app.compose_content.start_queued.contains(&i) {
                Style::default().fg(Color::Yellow)
            } else if app.compose_content.stop_queued.contains(&i) {
                Style::default().fg(Color::Red)
            } else if app.running_container_names.iter().any(|m| m.contains(s)) {
                Style::default().fg(Color::LightGreen)
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

    frame.render_stateful_widget(list, main_and_logs[0], &mut app.compose_content.state);

    let docker_modifiers = create_docker_modifiers(app.compose_content.modifiers);
    frame.render_widget(docker_modifiers, main_and_modifier[1]);

    let legend = create_legend();
    frame.render_widget(legend, main_and_legend[1]);
}
