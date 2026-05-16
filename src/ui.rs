use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, AppMode, HealthOperation, NewCreatureField};
use crate::creature::{Creature, CreatureId};

const ROW_HEIGHT: u16 = 3;

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    render_header(frame, header_area);
    render_creatures(app, frame, list_area);
    render_footer(frame, footer_area);
    render_popup(app, frame, area, list_area);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let columns = row_columns(area.inner(Margin {
        horizontal: 1,
        vertical: 0,
    }));
    let style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(Paragraph::new("Name").style(style), columns[0]);
    frame.render_widget(Paragraph::new("Init").style(style), columns[1]);
    frame.render_widget(Paragraph::new("AC").style(style), columns[2]);
    frame.render_widget(Paragraph::new("Health").style(style), columns[3]);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let help = "j/k move • Space select • +/- health • n new • u undo • Ctrl+R redo • q quit";
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn render_creatures(app: &mut App, frame: &mut Frame, area: Rect) {
    let visible_rows = (area.height / ROW_HEIGHT).max(1) as usize;
    app.ensure_hover_visible(visible_rows);

    for (visible_index, creature) in app
        .creatures
        .iter()
        .skip(app.scroll_offset)
        .take(visible_rows)
        .enumerate()
    {
        let y = area.y + visible_index as u16 * ROW_HEIGHT;
        if y >= area.bottom() {
            break;
        }
        let row_area = Rect::new(area.x, y, area.width, ROW_HEIGHT.min(area.bottom() - y));
        render_creature_row(
            app,
            frame,
            row_area,
            creature,
            app.scroll_offset + visible_index,
        );
    }
}

fn render_creature_row(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    creature: &Creature,
    display_index: usize,
) {
    let is_hovered = app.hovered == Some(display_index);
    let is_selected = app.selected.contains(&creature.id);
    let border_style = row_border_style(is_hovered, is_selected);

    if is_hovered || is_selected {
        frame.render_widget(Block::bordered().border_style(border_style), area);
    }

    let content_area = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    if content_area.is_empty() {
        return;
    }

    let columns = row_columns(content_area);
    let text_style = if creature.is_down() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::White)
    };
    let selected_marker = if is_selected { "● " } else { "  " };

    render_cell(
        frame,
        columns[0],
        Line::from(vec![
            Span::styled(selected_marker, border_style),
            Span::styled(creature.name.clone(), text_style),
        ]),
    );
    render_cell(
        frame,
        columns[1],
        Line::from(Span::styled(
            creature
                .initiative
                .map_or_else(|| "—".to_string(), |initiative| initiative.to_string()),
            text_style,
        )),
    );
    render_cell(
        frame,
        columns[2],
        Line::from(Span::styled(
            creature
                .ac
                .map_or_else(|| "—".to_string(), |ac| ac.to_string()),
            text_style,
        )),
    );
    render_cell(
        frame,
        columns[3],
        Line::from(Span::styled(
            format!("{}/{}", creature.get_health(), creature.get_max_health()),
            text_style,
        )),
    );
}

fn row_columns(area: Rect) -> [Rect; 4] {
    Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(18),
        Constraint::Percentage(12),
        Constraint::Fill(1),
    ])
    .areas(area)
}

fn render_cell(frame: &mut Frame, area: Rect, line: Line<'static>) {
    frame.render_widget(Paragraph::new(line), area);
}

fn row_border_style(is_hovered: bool, is_selected: bool) -> Style {
    match (is_hovered, is_selected) {
        (true, true) => Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
        (false, true) => Style::default().fg(Color::Green),
        (true, false) => Style::default().fg(Color::Cyan),
        (false, false) => Style::default().fg(Color::DarkGray),
    }
}

fn render_popup(app: &mut App, frame: &mut Frame, full_area: Rect, list_area: Rect) {
    match &mut app.mode {
        AppMode::Normal => {}
        AppMode::HealthInput(input) => {
            let target_label = target_label(&app.creatures, &input.target_ids);
            let title = match input.operation {
                HealthOperation::Add => format!("Add HP to {target_label}"),
                HealthOperation::Subtract => format!("Subtract HP from {target_label}"),
            };
            input.textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::LightBlue))
                    .title(title),
            );

            let area = health_popup_area(full_area, list_area, app.hovered, app.scroll_offset);
            frame.render_widget(Clear, area);
            frame.render_widget(&input.textarea, area);
            if let Some(error) = &input.error {
                let error_area = Rect::new(area.x, area.bottom(), area.width, 1).clamp(full_area);
                frame.render_widget(
                    Paragraph::new(error.clone()).style(Style::default().fg(Color::Red)),
                    error_area,
                );
            }
        }
        AppMode::NewCreature(form) => {
            style_new_creature_form(form);
            let area = full_area.centered(Constraint::Min(50), Constraint::Length(19));
            frame.render_widget(Clear, area);
            frame.render_widget(
                Block::bordered()
                    .title("New creature")
                    .border_style(Style::default().fg(Color::LightBlue)),
                area,
            );
            let inner = area.inner(Margin {
                horizontal: 1,
                vertical: 1,
            });
            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(inner);
            frame.render_widget(&form.fields.name, chunks[0]);
            frame.render_widget(&form.fields.initiative, chunks[1]);
            frame.render_widget(&form.fields.ac, chunks[2]);
            frame.render_widget(&form.fields.health, chunks[3]);
            frame.render_widget(&form.fields.count, chunks[4]);
            let message = form
                .error
                .clone()
                .unwrap_or_else(|| "Tab/Shift+Tab fields • Enter create • Esc cancel".to_string());
            let style = if form.error.is_some() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            frame.render_widget(
                Paragraph::new(message)
                    .style(style)
                    .wrap(Wrap { trim: true }),
                chunks[5],
            );
        }
    }
}

fn target_label(creatures: &crate::creature::Creatures, target_ids: &[CreatureId]) -> String {
    if target_ids.len() == 1 {
        target_ids
            .first()
            .and_then(|id| creatures.get(*id))
            .map_or_else(|| "creature".to_string(), |creature| creature.name.clone())
    } else {
        format!("{} creatures", target_ids.len())
    }
}

fn health_popup_area(
    full_area: Rect,
    list_area: Rect,
    hovered: Option<usize>,
    scroll_offset: usize,
) -> Rect {
    let width = full_area.width.min(36);
    let height = 3;
    let x = full_area.x + full_area.width.saturating_sub(width) / 2;
    let y = hovered
        .and_then(|hovered| hovered.checked_sub(scroll_offset))
        .map(|visible| list_area.y + visible as u16 * ROW_HEIGHT)
        .unwrap_or(full_area.y + full_area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width, height).clamp(full_area)
}

fn style_new_creature_form(form: &mut crate::app::NewCreatureForm) {
    set_field_block(
        &mut form.fields.name,
        "name",
        form.active_field == NewCreatureField::Name,
    );
    set_field_block(
        &mut form.fields.initiative,
        "initiative",
        form.active_field == NewCreatureField::Initiative,
    );
    set_field_block(
        &mut form.fields.ac,
        "AC",
        form.active_field == NewCreatureField::Ac,
    );
    set_field_block(
        &mut form.fields.health,
        "health",
        form.active_field == NewCreatureField::Health,
    );
    set_field_block(
        &mut form.fields.count,
        "count",
        form.active_field == NewCreatureField::Count,
    );
}

fn set_field_block(
    textarea: &mut ratatui_textarea::TextArea<'static>,
    title: &'static str,
    active: bool,
) {
    let style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::LightBlue)
    };
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title),
    );
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, style::Color};

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
}
