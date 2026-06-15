use crate::app::{App, FocusedElement};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(frame.area());

    draw_title(frame, chunks[0]);
    draw_main_content(frame, chunks[1], app);
    draw_status_bar(frame, chunks[2], app);
}

fn draw_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("DDC/CI Monitor Control")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
}

fn draw_main_content(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    draw_monitor_list(frame, chunks[0], app);
    draw_controls(frame, chunks[1], app);
}

fn draw_monitor_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = if i == app.selected_monitor_idx {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(m.name.clone()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Monitors"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(list, area);
}

fn draw_controls(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let is_brightness_focused = app.focused_element == FocusedElement::Brightness;
    let brightness_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Brightness (0x10)"),
        )
        .gauge_style(
            Style::default()
                .fg(if is_brightness_focused {
                    Color::Yellow
                } else {
                    Color::Green
                })
                .add_modifier(if is_brightness_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        )
        .percent((app.current_brightness as f64 / app.max_brightness as f64 * 100.0) as u16)
        .label(format!("{}/{}", app.current_brightness, app.max_brightness));
    frame.render_widget(brightness_gauge, chunks[0]);

    let is_contrast_focused = app.focused_element == FocusedElement::Contrast;
    let contrast_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Contrast (0x12)"),
        )
        .gauge_style(
            Style::default()
                .fg(if is_contrast_focused {
                    Color::Yellow
                } else {
                    Color::Green
                })
                .add_modifier(if is_contrast_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        )
        .percent((app.current_contrast as f64 / app.max_contrast as f64 * 100.0) as u16)
        .label(format!("{}/{}", app.current_contrast, app.max_contrast));
    frame.render_widget(contrast_gauge, chunks[1]);

    let help_text = vec![Line::from(vec![
        Span::styled("↑/↓ ", Style::default().fg(Color::Yellow)),
        Span::raw("Navigate  "),
        Span::styled("←/→ ", Style::default().fg(Color::Yellow)),
        Span::raw("Adjust  "),
        Span::styled("r ", Style::default().fg(Color::Yellow)),
        Span::raw("Refresh  "),
        Span::styled("q ", Style::default().fg(Color::Yellow)),
        Span::raw("Quit"),
    ])];
    let help =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Controls"));
    frame.render_widget(help, chunks[2]);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let style = if app.is_error {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let status = Paragraph::new(app.status_message.clone())
        .style(style)
        .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(status, area);
}
