use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListDirection, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, StatefulWidget, Widget, Wrap,
    },
    Frame,
};
use ratatui_macros::vertical;

use crate::app::{App, DockerModifier};

fn create_legend<'a>() -> Paragraph<'a> {
    let text = Line::default().spans(vec![
        Span::styled(
            "(Enter)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" start selected, "),
        Span::styled(
            "(a)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" start all containers, "),
        Span::styled(
            "(s)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" stop selected, "),
        Span::styled(
            "(x)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" stop all containers, "),
        Span::styled(
            "(r)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" restart container, "),
        Span::styled(
            "(PageUp/j)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" scroll up, "),
        Span::styled(
            "(PageDown/k)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" scroll down "),
    ]);

    let bottom_line = Line::default().spans(vec![
        Span::styled(
            "(ctrl + l)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" clear logs, "),
        Span::styled(
            "(ctrl + w)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" remove container with volumes, "),
        Span::styled(
            "(ctrl+ alt + w)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" remove all containers with volumes, "),
        Span::styled(
            "(q)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" to quit."),
    ]);

    Paragraph::new(vec![text, bottom_line]).block(
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
    if size.width < MIN_COLS || size.height < MIN_ROWS {
        frame.render_widget(ResizeScreen::new(), frame.area());
        return;
    }

    let main_and_legend = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(4)])
        .split(size);

    let main_and_modifier = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(main_and_legend[0]);

    let main_and_logs = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(main_and_modifier[0]);

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
            textwrap::Options::new(main_and_logs[1].width.saturating_sub(3) as _),
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
                    .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
            )
            .scroll((app.vertical_scroll as _, 0)),
        main_and_logs[1],
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
                .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
        );

    frame.render_stateful_widget(list, main_and_logs[0], &mut app.compose_content.state);

    let docker_modifiers = create_docker_modifiers(app.compose_content.modifiers);
    frame.render_widget(docker_modifiers, main_and_modifier[1]);

    let legend = create_legend();
    frame.render_widget(legend, main_and_legend[1]);

    let content = app.compose_content.error_msg.as_deref().unwrap_or_default();

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    frame.render_stateful_widget(
        scrollbar,
        main_and_logs[1].inner(Margin {
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
            .style(Style::new().light_blue().bg(Color::Black))
            .title("Error")
            .title_style(Style::new().black().bold())
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

use derive_setters::Setters;

#[derive(Debug, Default, Setters)]
struct Popup<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    border_style: Style,
    title_style: Style,
    style: Style,
}

impl StatefulWidget for Popup<'_> {
    type State = usize;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);
        let block = Block::new()
            .title(self.title)
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(self.border_style);
        Paragraph::new(self.content)
            .scroll((*state as _, 0))
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(block)
            .render(area, buf);
    }
}

const MIN_ROWS: u16 = 20;
const MIN_COLS: u16 = 100;

#[derive(Debug)]
pub struct ResizeScreen {
    pub min_height: u16,
    pub min_width: u16,
}

impl ResizeScreen {
    pub fn new() -> Self {
        Self {
            min_width: MIN_COLS,
            min_height: MIN_ROWS,
        }
    }
}

impl Widget for ResizeScreen {
    fn render(self, area: ratatui::prelude::Rect, buffer: &mut Buffer) {
        let original_height = area.height;
        let original_width = area.width;

        let mut height_span = Span::from(format!("{}", original_height));

        let height_style = if original_height >= self.min_height {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };
        height_span = height_span.style(height_style);

        let mut width_span = Span::from(format!("{}", original_width));

        let width_style = if original_width >= self.min_width {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };
        width_span = width_span.style(width_style);

        let messages = vec![
            Line::from("Terminal too small; current size:"),
            Line::from(vec![
                Span::from("Width = "),
                width_span,
                Span::from(", ".to_string()),
                Span::from("Height = "),
                height_span,
            ]),
            Line::from(""),
            Line::from("Required dimensions:"),
            Line::from(vec![
                Span::from(format!("Width = {}", self.min_width)),
                Span::from(", ".to_string()),
                Span::from(format!("Height = {}", self.min_height)),
            ]),
        ];

        let [_, inner_area, _] = vertical![>=0, <=5, >=0].areas(area);
        Text::from(messages)
            .alignment(ratatui::layout::Alignment::Center)
            .render(inner_area, buffer);

        Block::bordered()
            .title("< Terminal Too Small >")
            .border_style(Style::default().fg(Color::Red))
            .render(area, buffer);
    }
}
