use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, List, ListDirection, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation,
    },
    Frame,
};

use crate::app::App;

use super::{
    get_bg_color,
    legend::{create_container_info, create_docker_modifiers, create_legend},
    popup::Popup,
};

pub fn render_main_screen(app: &mut App, frame: &mut Frame) {
    let bg = get_bg_color();
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
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(main_and_modifier[0]);

    let logs_and_info = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(main_and_logs[1]);
    frame.render_widget(create_container_info(app), logs_and_info[1]);

    let content = app
        .compose_content
        .logs
        .lock()
        .unwrap()
        .get(&app.compose_content.state.selected().unwrap_or(0))
        .cloned()
        .unwrap_or_default();
    app.vertical_scroll_state = app
        .vertical_scroll_state
        .viewport_content_length(20)
        .content_length(content.len());
    let wrapped = Text::from(
        textwrap::wrap(
            &content.join(""),
            // Terminating 3 pixels before is a bit nicer
            textwrap::Options::new(logs_and_info[0].width.saturating_sub(3) as _),
        )
        .iter()
        .map(|s| Line::from(s.to_string()))
        .collect::<Vec<_>>(),
    );
    frame.render_widget(
        Paragraph::new(wrapped)
            .block(
                Block::bordered()
                    .title("Logs")
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(Color::LightBlue).bg(bg)),
            )
            .scroll((app.vertical_scroll as _, 0)),
        logs_and_info[0],
    );

    let items: Vec<ListItem> = app
        .compose_content
        .compose
        .services
        .0
        .keys()
        .enumerate()
        .zip(app.container_name_mapping.values())
        .map(|((i, display_name), real_name)| {
            let content = Text::raw(display_name);
            let style = if app.compose_content.start_queued.state.contains(&i) {
                Style::default().fg(Color::Yellow)
            } else if app.compose_content.stop_queued.state.contains(&i) {
                Style::default().fg(Color::Red)
            } else if app.running_container_names.iter().any(|m| m == real_name) {
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
                .title("Docker Compose TUI")
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::LightBlue).bg(bg)),
        );

    frame.render_stateful_widget(list, main_and_logs[0], &mut app.compose_content.state);

    let docker_modifiers = create_docker_modifiers(app.compose_content.modifiers);
    frame.render_widget(docker_modifiers, main_and_modifier[1]);

    let legend = create_legend(app);
    frame.render_widget(legend, main_and_legend[1]);

    let content = app.compose_content.error_msg.as_deref().unwrap_or_default();

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    frame.render_stateful_widget(
        scrollbar,
        logs_and_info[0].inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.vertical_scroll_state,
    );
    if app.show_popup {
        let area = frame.area();

        let popup_area = Rect {
            x: area.width / 16,
            y: area.height / 12,
            width: area.width / 8 * 7,
            height: area.height / 8 * 5,
        };
        let wrapped = Text::from(
            textwrap::wrap(
                content,
                textwrap::Options::new(popup_area.width.saturating_sub(3) as _),
            )
            .iter()
            .map(|s| Line::from(s.to_string()))
            .collect::<Vec<_>>(),
        );
        app.popup_scroll_state = app
            .popup_scroll_state
            .viewport_content_length(20)
            .content_length(wrapped.height());

        let popup = Popup::default()
            .content(wrapped)
            .style(Style::new().light_blue().bg(bg))
            .title("Error")
            .title_style(Style::new().white().bold())
            .border_style(Style::new().red());

        frame.render_stateful_widget(popup, popup_area, &mut app.popup_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            popup_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut app.popup_scroll_state,
        );
    }
}
