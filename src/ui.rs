use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Row, Table};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    let header = Row::new(["Name", "Initiative", "Health"])
        .style(Style::new().bold())
        .bottom_margin(1);

    let rows = app.creatures.iter().map(|c| {
        Row::new([
            c.name.clone(),
            c.initiative.to_string(),
            c.get_health().to_string(),
        ])
    });

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(20),
        Constraint::Percentage(50),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .style(Color::White)
        .row_highlight_style(Style::new().on_black().bold())
        .column_highlight_style(Color::Gray)
        .cell_highlight_style(Style::new().reversed().cyan())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, frame.area(), &mut app.creature_table_state)
}
