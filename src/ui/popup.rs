use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::{
    app::App,
    input::{AppMode, HealthOperation, NewCreatureField},
};

use super::encounter::{ROW_HEIGHT, target_label};

pub(crate) fn render_popup(app: &mut App, frame: &mut Frame, full_area: Rect, list_area: Rect) {
    match &mut app.mode {
        AppMode::Normal => {}
        AppMode::HealthInput(input) => {
            let target_label = target_label(&app.creatures, &input.target_ids);
            let title = match input.operation {
                HealthOperation::Add => format!("Add HP to {target_label}"),
                HealthOperation::Subtract => format!("Subtract HP from {target_label}"),
            };
            input.textarea.set_block(input_block(title));

            let area = row_popup_area(full_area, list_area, app.hovered, app.scroll_offset);
            render_textarea_popup(
                frame,
                full_area,
                area,
                &input.textarea,
                input.error.as_deref(),
            );
        }
        AppMode::InitiativeInput(input) => {
            let target_label = target_label(&app.creatures, &input.target_ids);
            input
                .textarea
                .set_block(input_block(format!("Set initiative for {target_label}")));

            let area = row_popup_area(full_area, list_area, app.hovered, app.scroll_offset);
            render_textarea_popup(
                frame,
                full_area,
                area,
                &input.textarea,
                input.error.as_deref(),
            );
        }
        AppMode::RenameInput(input) => {
            let title = app.creatures.get(input.target_id).map_or_else(
                || "Rename creature".to_string(),
                |creature| format!("Rename {}", creature.name),
            );
            input.textarea.set_block(input_block(title));

            let area = row_popup_area(full_area, list_area, app.hovered, app.scroll_offset);
            render_textarea_popup(
                frame,
                full_area,
                area,
                &input.textarea,
                input.error.as_deref(),
            );
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

fn input_block(title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightBlue))
        .title(title)
}

fn render_textarea_popup(
    frame: &mut Frame,
    full_area: Rect,
    area: Rect,
    textarea: &ratatui_textarea::TextArea<'_>,
    error: Option<&str>,
) {
    frame.render_widget(Clear, area);
    frame.render_widget(textarea, area);
    if let Some(error) = error {
        let error_area = Rect::new(area.x, area.bottom(), area.width, 1).clamp(full_area);
        frame.render_widget(
            Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red)),
            error_area,
        );
    }
}

fn row_popup_area(
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

fn style_new_creature_form(form: &mut crate::input::NewCreatureForm) {
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
