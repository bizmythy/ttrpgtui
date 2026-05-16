use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::{
    app::App,
    creature::{Creature, CreatureId},
};

pub(crate) const ROW_HEIGHT: u16 = 3;

pub(crate) fn render_header(frame: &mut Frame, area: Rect) {
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

pub(crate) fn render_footer(frame: &mut Frame, area: Rect) {
    let help = "j/k move • Space select • +/- health • i initiative • n new • r rename • u undo • Ctrl+R redo • q quit";
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

pub(crate) fn render_creatures(app: &mut App, frame: &mut Frame, area: Rect) {
    let visible_rows = visible_rows(area);
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

pub(crate) fn visible_rows(area: Rect) -> usize {
    (area.height / ROW_HEIGHT).max(1) as usize
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
            Span::styled(
                creature.name.clone(),
                text_style.add_modifier(Modifier::BOLD),
            ),
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
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        (false, true) => Style::default().fg(Color::Green),
        (true, false) => Style::default().fg(Color::Cyan),
        (false, false) => Style::default().fg(Color::DarkGray),
    }
}

pub(crate) fn target_label(
    creatures: &crate::creature::Creatures,
    target_ids: &[CreatureId],
) -> String {
    if target_ids.len() == 1 {
        target_ids
            .first()
            .and_then(|id| creatures.get(*id))
            .map_or_else(|| "creature".to_string(), |creature| creature.name.clone())
    } else {
        format!("{} creatures", target_ids.len())
    }
}
