use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    app::{App, DockerModifier},
    utils::shorten_path,
};

pub fn create_legend(app: &App) -> Paragraph<'_> {
    let content = Line::from(vec![
        Span::raw("Project name: "),
        Span::styled(
            app.project_name.as_str(),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" File: "),
        Span::styled(
            shorten_path(app.full_path.as_path())
                .to_string_lossy()
                .into_owned(),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" Docker version: "),
        Span::styled(
            &app.docker_version,
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
    ]);

    Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title("General")
            .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
    )
}

pub fn create_docker_modifiers(modifiers: DockerModifier) -> Paragraph<'static> {
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

pub fn create_container_info(app: &mut App) -> impl Widget + '_ {
    let selected = app.compose_content.state.selected().unwrap();
    let Some(Some(container_info)) = app.container_info.get(&selected) else {
        return Paragraph::new(Line::styled(
            "Not available/Not running",
            Style::default().fg(Color::Red),
        ))
        .block(
            Block::default()
                .title("Container info")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
        );
    };
    let value_style = Style::default().fg(Color::LightYellow);

    let name = container_info.name.as_deref().unwrap_or_default();
    let created = container_info.created.as_deref().unwrap_or_default();

    let image = container_info
        .config
        .as_ref()
        .and_then(|c| c.image.as_deref())
        .unwrap_or_default();
    let num_of_volumes = container_info
        .config
        .as_ref()
        .and_then(|c| c.volumes.as_ref().map(|v| v.len()))
        .unwrap_or_default();
    let state = container_info
        .state
        .as_ref()
        .and_then(|state| state.status.map(|status| status.to_string()))
        .unwrap_or_else(|| String::from("unknown"));

    let content = Line::from(vec![
        Span::raw("image: "),
        Span::styled(image, value_style),
        Span::raw(" name: "),
        Span::styled(name, value_style),
        Span::raw(" created: "),
        Span::styled(created, value_style),
        Span::raw(" state: "),
        Span::styled(state, value_style),
        Span::raw(" attached volumes: "),
        Span::styled(num_of_volumes.to_string(), value_style),
    ]);
    Paragraph::new(content).block(
        Block::default()
            .title("Container info")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
    )
}
