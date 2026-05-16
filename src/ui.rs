use color_eyre::Result;
use crossterm::event::{self, KeyCode};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table, TableState};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    let header = Row::new(["Name", "Initiative", "Health"])
        .style(Style::new().bold())
        .bottom_margin(1);

    let rows = [
        Row::new(["Eggplant", "1 medium", "25 kcal, 6g carbs, 1g protein"]),
        Row::new(["Tomato", "2 large", "44 kcal, 10g carbs, 2g protein"]),
        Row::new(["Zucchini", "1 medium", "33 kcal, 7g carbs, 2g protein"]),
        Row::new(["Bell Pepper", "1 medium", "24 kcal, 6g carbs, 1g protein"]),
        Row::new(["Garlic", "2 cloves", "9 kcal, 2g carbs, 0.4g protein"]),
    ];

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
