use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_macros::vertical;

use crate::text_wrap::{wrap_line, Options};

use super::get_bg_color;

pub fn render_help(frame: &mut Frame) {
    let bg = get_bg_color();
    let [_, inner_area, _] = vertical![>=0, <=7, >=0].areas(frame.area());
    frame.render_widget(
        Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::LightBlue).bg(bg)),
        frame.area(),
    );
    let text = Line::default().spans(vec![
        Span::styled(
            "Basic ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ),
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
        Span::raw(" restart selected"),
    ]);

    let navigation = Line::default().spans(vec![
        Span::styled(
            "Navigation ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ),
        Span::styled(
            "(Mouse scroll up/PageUp/j) ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw("scroll up, "),
        Span::styled(
            "(Mouse scroll down/PageDown/k) ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw("scroll down, "),
        Span::styled(
            "↓ / ↑ (shift + ↓) / (shift + ↑) ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw("navigate container list (jump to first / last), "),
        Span::styled(
            "(e) ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw("enter alternate screen, "),
        Span::styled(
            "(tab)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" move focus on alternate screen"),
    ]);

    let bottom_line = Line::default().spans(vec![
        Span::styled(
            "Meta ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ),
        Span::styled(
            "(f)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" force refresh, "),
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

    let mut text = wrap_line(
        &text,
        Options::from_width_and_header(inner_area.width as _, "Basic"),
    );
    let navigation = wrap_line(
        &navigation,
        Options::from_width_and_header(inner_area.width as _, "Navigation"),
    );
    let bottom_line = wrap_line(
        &bottom_line,
        Options::from_width_and_header(inner_area.width as _, "Meta"),
    );

    text.lines.extend(navigation.lines);
    text.lines.extend(bottom_line.lines);

    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Keys")
                .style(Style::default().fg(Color::LightBlue).bg(bg)),
        ),
        inner_area,
    );
}
