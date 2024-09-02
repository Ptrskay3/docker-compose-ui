use ratatui::{
    buffer::Buffer,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Widget},
};
use ratatui_macros::vertical;

use super::{MIN_COLS, MIN_ROWS};

#[derive(Debug)]
pub struct ResizeScreen {
    pub min_height: u16,
    pub min_width: u16,
}

impl Default for ResizeScreen {
    fn default() -> Self {
        Self::new()
    }
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
