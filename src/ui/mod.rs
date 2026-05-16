mod encounter;
mod popup;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    encounter::render_header(frame, header_area);
    encounter::render_creatures(app, frame, list_area);
    encounter::render_footer(frame, footer_area);
    popup::render_popup(app, frame, area, list_area);
}

#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        style::{Color, Modifier},
    };

    use super::render;
    use crate::app::App;

    #[test]
    fn hovered_row_draws_border() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal.draw(|frame| render(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 1)].symbol(), "┌");
        assert_eq!(buffer[(79, 1)].symbol(), "┐");
    }

    #[test]
    fn down_creature_renders_red() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        let id = app.hovered_id().unwrap();
        app.creatures.get_mut(id).unwrap().modify_health(-100);

        terminal.draw(|frame| render(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        let john_x = 3;
        let content_y = 2;
        assert_eq!(buffer[(john_x, content_y)].fg, Color::Red);
    }

    #[test]
    fn creature_names_render_bold() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal.draw(|frame| render(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        let first_name_x = 3;
        let first_name_y = 2;
        assert!(
            buffer[(first_name_x, first_name_y)]
                .modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn hovered_selected_row_uses_cyan_border() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.toggle_hovered_selection();

        terminal.draw(|frame| render(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 1)].fg, Color::Cyan);
    }
}
