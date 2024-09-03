mod container_details;
mod help;
mod legend;
mod main_screen;
mod popup;
mod resize_screen;

use ratatui::{style::Color, Frame};

use crate::{app::App, handler::AlternateScreenContent, LIGHT_MODE};

const UNNAMED: &str = "<unnamed>";
const UNSPECIFIED: &str = "<unspecified>";
const ALL_INTERFACES: &str = "0.0.0.0";
const MIN_ROWS: u16 = 20;
const MIN_COLS: u16 = 130;

const BG_LIGHT: Color = Color::White;
const BG_DARK: Color = Color::Black;

pub fn render(app: &mut App, frame: &mut Frame) {
    let size = frame.area();
    if size.width < MIN_COLS || size.height < MIN_ROWS {
        frame.render_widget(resize_screen::ResizeScreen::new(), frame.area());
        return;
    }
    match app.alternate_screen_content {
        AlternateScreenContent::Help => help::render_help(frame),

        AlternateScreenContent::ContainerDetails(i) => {
            container_details::render_container_details(app, frame, i)
        }

        AlternateScreenContent::None => main_screen::render_main_screen(app, frame),
    }
}

pub fn get_bg_color() -> Color {
    if *LIGHT_MODE.get().unwrap() {
        BG_LIGHT
    } else {
        BG_DARK
    }
}
