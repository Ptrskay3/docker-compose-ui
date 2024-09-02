use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation},
    Frame,
};
use ratatui_macros::{horizontal, vertical};

use super::{legend::create_container_info, ALL_INTERFACES, UNNAMED, UNSPECIFIED};
use crate::{app::App, handler::SplitScreen};

pub fn render_container_details(app: &mut App, frame: &mut Frame, i: SplitScreen) {
    let size = frame.area();
    let selected = app
        .compose_content
        .state
        .selected()
        .expect("a valid selection");
    let Some(Some(container_info)) = app.container_info.get(&selected) else {
        let name = app.container_name_mapping.get(&selected).expect("to exist");
        frame.render_widget(
            Paragraph::new(Line::default().spans(vec![
                Span::raw("We don't know anything interesting about "),
                Span::styled(name, Style::default().fg(Color::Red)),
                Span::raw(" yet.. Have you tried starting it?"),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::LightBlue).bg(Color::Black)),
            ),
            frame.area(),
        );
        return;
    };
    let env = container_info
        .config
        .as_ref()
        .and_then(|cfg| cfg.env.as_deref())
        .unwrap_or_default();

    let labels = container_info
        .config
        .as_ref()
        .and_then(|cfg| cfg.labels.clone())
        .unwrap_or_default();

    let labels_formatted: Vec<_> = labels
        .into_iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect();

    let volumes = container_info
        .mounts
        .as_ref()
        .map(|mounts| {
            mounts
                .iter()
                .enumerate()
                .map(|(i, mount)| {
                    format!(
                        "{}:\n name: {}\n source: {}\n destination: {}\n driver: {}",
                        i + 1,
                        mount.name.as_deref().unwrap_or(UNNAMED),
                        mount.source.as_deref().unwrap_or_default(),
                        mount.destination.as_deref().unwrap_or_default(),
                        mount.driver.as_deref().unwrap_or(UNSPECIFIED),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut networks = container_info
        .host_config
        .as_ref()
        .and_then(|cfg| {
            cfg.port_bindings.as_ref().map(|ports| {
                let mut result = vec![String::from("Port bindings:")];
                ports.iter().for_each(|(port, bindings)| {
                    if let Some(bindings) = bindings {
                        for binding in bindings {
                            let host_ip = match binding.host_ip.as_deref() {
                                Some("") | None => ALL_INTERFACES,
                                Some(ip) => ip,
                            };

                            result.push(format!(
                                " {port} -> {}:{}",
                                host_ip,
                                binding.host_port.as_deref().unwrap_or_default()
                            ));
                        }
                    } else {
                        result.push(format!(" {port}"));
                    }
                });
                result
            })
        })
        .unwrap_or_default();

    let network_settings = container_info
        .network_settings
        .as_ref()
        .map(|settings| {
            let mut result = vec![String::from("Network descriptions:")];
            let network_descriptions = settings.networks.iter().flat_map(|network| {
                network
                    .iter()
                    .enumerate()
                    .map(|(i, (name, endpoint))| {
                        format!(
                            " {}:\n  name: {}\n  ipv4_address: {}\n  id: {}\n",
                            i + 1,
                            name,
                            endpoint
                                .ipam_config
                                .as_ref()
                                .and_then(|i| i.ipv4_address.as_deref())
                                .unwrap_or(UNSPECIFIED),
                            endpoint.network_id.as_deref().unwrap_or(UNSPECIFIED),
                        )
                    })
                    .collect::<Vec<_>>()
            });

            result.extend(network_descriptions);
            result
        })
        .unwrap_or_default();
    networks.extend(network_settings);

    let header_and_main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(size);

    let [upper_area, lower_area] = vertical![== 50%, == 50%].areas(header_and_main[1]);
    let [upper_left, upper_right] = horizontal![== 50%, == 50%].areas(upper_area);
    let [lower_left, lower_right] = horizontal![== 50%, == 50%].areas(lower_area);

    let style_selected = Style::default().fg(Color::Red).bg(Color::Black);
    let style_not_selected = Style::default().fg(Color::LightBlue).bg(Color::Black);
    let (label_style, env_style, volume_style, network_style) = match i {
        SplitScreen::UpperLeft => (
            style_selected,
            style_not_selected,
            style_not_selected,
            style_not_selected,
        ),
        SplitScreen::LowerLeft => (
            style_not_selected,
            style_selected,
            style_not_selected,
            style_not_selected,
        ),
        SplitScreen::UpperRight => (
            style_not_selected,
            style_not_selected,
            style_selected,
            style_not_selected,
        ),
        SplitScreen::LowerRight => (
            style_not_selected,
            style_not_selected,
            style_not_selected,
            style_selected,
        ),
    };

    app.alternate_screen.lower_left_scroll_state = app
        .alternate_screen
        .lower_left_scroll_state
        .viewport_content_length(20)
        .content_length(env.len());
    app.alternate_screen.upper_left_scroll_state = app
        .alternate_screen
        .upper_left_scroll_state
        .viewport_content_length(20)
        .content_length(labels_formatted.len());
    app.alternate_screen.lower_right_scroll_state = app
        .alternate_screen
        .lower_right_scroll_state
        .viewport_content_length(20)
        .content_length(networks.len());
    app.alternate_screen.upper_right_scroll_state = app
        .alternate_screen
        .upper_right_scroll_state
        .viewport_content_length(20)
        .content_length(volumes.len());

    let networks = Text::from(
        textwrap::wrap(
            &networks.join("\n"),
            textwrap::Options::new(lower_right.width.saturating_sub(2) as _),
        )
        .iter()
        .map(|s| Line::from(s.to_string()))
        .collect::<Vec<_>>(),
    );
    let labels_formatted = Text::from(
        textwrap::wrap(
            &labels_formatted.join("\n"),
            textwrap::Options::new(upper_left.width.saturating_sub(2) as _),
        )
        .iter()
        .map(|s| Line::from(s.to_string()))
        .collect::<Vec<_>>(),
    );

    let volumes = Text::from(
        textwrap::wrap(
            &volumes.join("\n"),
            textwrap::Options::new(upper_right.width.saturating_sub(2) as _),
        )
        .iter()
        .map(|s| Line::from(s.to_string()))
        .collect::<Vec<_>>(),
    );

    let env = Text::from(
        textwrap::wrap(
            &env.join("\n"),
            textwrap::Options::new(lower_left.width.saturating_sub(2) as _),
        )
        .iter()
        .map(|s| Line::from(s.to_string()))
        .collect::<Vec<_>>(),
    );

    frame.render_widget(
        Paragraph::new(env)
            .scroll((app.alternate_screen.lower_left_scroll as _, 0))
            .block(
                Block::default()
                    .title("Environment variables")
                    .borders(Borders::ALL)
                    .style(env_style),
            ),
        lower_left,
    );
    frame.render_widget(
        Paragraph::new(networks)
            .scroll((app.alternate_screen.lower_right_scroll as _, 0))
            .block(
                Block::default()
                    .title("Networks")
                    .borders(Borders::ALL)
                    .style(network_style),
            ),
        lower_right,
    );

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    frame.render_stateful_widget(
        scrollbar,
        lower_left.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.alternate_screen.lower_left_scroll_state,
    );
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    frame.render_stateful_widget(
        scrollbar,
        lower_right.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.alternate_screen.lower_right_scroll_state,
    );

    frame.render_widget(
        Paragraph::new(labels_formatted)
            .scroll((app.alternate_screen.upper_left_scroll as _, 0))
            .block(
                Block::default()
                    .title("Labels")
                    .borders(Borders::ALL)
                    .style(label_style),
            ),
        upper_left,
    );
    frame.render_widget(
        Paragraph::new(volumes)
            .scroll((app.alternate_screen.upper_right_scroll as _, 0))
            .block(
                Block::default()
                    .title("Volumes")
                    .borders(Borders::ALL)
                    .style(volume_style),
            ),
        upper_right,
    );
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    frame.render_stateful_widget(
        scrollbar,
        upper_right.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.alternate_screen.upper_right_scroll_state,
    );
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    frame.render_stateful_widget(
        scrollbar,
        upper_left.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.alternate_screen.upper_left_scroll_state,
    );

    frame.render_widget(create_container_info(app), header_and_main[0]);
}
