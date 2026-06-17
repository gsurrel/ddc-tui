use crate::app::{App, FocusArea, VcpFeatureInfo};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};

/// Top-level draw entry
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < 40 || area.height < 10 {
        let p = Paragraph::new(Line::from(vec![Span::styled(
            "Terminal too small.",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(6),    // Body (left + right)
            Constraint::Length(4), // Merged Status Controls
        ])
        .split(area);

    draw_title(frame, main_chunks[0]);
    draw_body(frame, main_chunks[1], app);
    draw_status_controls(frame, main_chunks[2], app);
}

fn draw_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![Span::styled(
        "DDC/CI Monitor Control",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
}

fn draw_body(frame: &mut Frame, area: Rect, app: &mut App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(10)])
        .split(area);
    draw_monitor_list(frame, cols[0], app);
    draw_monitor_controls(frame, cols[1], app);
}

fn draw_monitor_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let mut spans = vec![Span::raw(m.name.clone())];
            if area.width > 20 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("({})", m.id),
                    Style::default().fg(Color::Gray),
                ));
            }
            let style = if app.focus_area == FocusArea::MonitorList && i == app.selected_monitor_idx
            {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(Span::styled(
        "Monitors",
        if app.focus_area == FocusArea::MonitorList {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        },
    )));
    frame.render_widget(list, area);
}

/// Right column: selected monitor block title includes the monitor name
fn draw_monitor_controls(frame: &mut Frame, area: Rect, app: &mut App) {
    let monitor_name = app
        .monitors
        .get(app.selected_monitor_idx)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "<no monitor>".into());
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(format!("Selected Monitor: {}", monitor_name));
    frame.render_widget(outer, area);
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if inner.width < 10 || inner.height < 3 {
        frame.render_widget(
            Paragraph::new("Not enough space").block(Block::default()),
            inner,
        );
        return;
    }
    let features = app
        .monitors
        .get(app.selected_monitor_idx)
        .map(|m| m.features.clone())
        .unwrap_or_default();
    draw_compact_features(frame, inner, app, &features);
}

/// Render a horizontal sequence of "pills" (small selectable labels).
fn render_pills(area: Rect, options: &'_ [String], selected: usize, is_focused: bool) -> Line<'_> {
    let mut spans: Vec<Span> = Vec::new();
    let mut used = 0usize;
    let max_width = area.width as usize;
    let mut first = true;
    for (i, opt) in options.iter().enumerate() {
        let pill_text = format!(" {} ", opt);
        let pill_len = pill_text.len();

        let sep = if first { 0 } else { 1 };
        if used + sep + pill_len > max_width.saturating_sub(3) {
            if !first {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled("…", Style::default().fg(Color::Gray)));
            break;
        }

        if !first {
            spans.push(Span::raw(" "));
            used += 1;
        }

        let style = if i == selected {
            if is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Black).bg(Color::Green)
            }
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };

        spans.push(Span::styled(pill_text, style));
        used += pill_len;
        first = false;
    }

    Line::from(spans)
}

/// Compact label:gauge layout for features with discrete handling and pill selectors
fn draw_compact_features(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
    features: &[VcpFeatureInfo],
) {
    if features.is_empty() {
        frame.render_widget(
            Paragraph::new("No supported VCP features found.").block(Block::default()),
            area,
        );
        return;
    }
    let max_rows = area.height as usize;
    if max_rows == 0 {
        return;
    }
    let features_to_draw = &features[..features.len().min(max_rows)];
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            features_to_draw
                .iter()
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>(),
        )
        .split(area);

    for (i, feature) in features_to_draw.iter().enumerate() {
        let row = rows[i];

        if row.width < 10 {
            continue;
        }

        // label/gauge split
        let mut label_width = (row.width as f32 * 0.35).max(12.0) as u16;
        if label_width + 6 >= row.width {
            label_width = row.width.saturating_sub(6);
        }
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(label_width), Constraint::Min(10)])
            .split(row);

        let is_focused = app.focus_area == FocusArea::VcpFeatures && i == app.selected_vcp_idx;
        let label_text = if feature.is_discrete {
            // For discrete, show name only (value shown as pills)
            format!("{}", feature.name)
        } else {
            // For continuous label shows current/max
            format!(
                "{} (0x{:02X}): {}/{}",
                feature.name, feature.code, feature.current, feature.max
            )
        };

        let label_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(label_text, label_style)])),
            cols[0],
        );

        if feature.is_discrete {
            // Build option labels and selected index
            let selected_idx = feature
                .option_values
                .iter()
                .position(|&v| v == feature.current)
                .unwrap_or(0);
            frame.render_widget(
                Paragraph::new(render_pills(
                    cols[1],
                    &feature.options,
                    selected_idx,
                    is_focused,
                )),
                cols[1],
            );
        } else {
            let ratio = if feature.max == 0 {
                0.0
            } else {
                (feature.current as f64 / feature.max as f64).clamp(0.0, 1.0)
            };
            let gauge_style = Style::default().fg(if is_focused {
                Color::Yellow
            } else {
                Color::Green
            });

            frame.render_widget(
                Gauge::default()
                    .gauge_style(gauge_style.add_modifier(if is_focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }))
                    .ratio(ratio)
                    .label(format!("{}/{}", feature.current, feature.max)),
                cols[1],
            );
        }
    }
}

/// Status + Controls block
fn draw_status_controls(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .title("Status Controls");
    frame.render_widget(outer, area);
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner.width < 10 || inner.height < 1 {
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if inner.height > 1 {
            vec![Constraint::Length(1), Constraint::Min(1)]
        } else {
            vec![Constraint::Length(1)]
        })
        .split(inner);
    let focus_text = match app.focus_area {
        FocusArea::MonitorList => "Focus: Monitors",
        FocusArea::VcpFeatures => "Focus: Controls",
    };

    let status_line = Line::from(vec![
        Span::styled(
            app.status_message.clone(),
            if app.is_error {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        ),
        Span::raw("  "),
        Span::styled(
            focus_text,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(status_line), chunks[0]);

    if chunks.len() > 1 {
        let help_line = Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate   "),
            Span::styled("←/→", Style::default().fg(Color::Yellow)),
            Span::raw(" Adjust   "),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" Switch Focus   "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh   "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit"),
        ]);

        frame.render_widget(
            Paragraph::new(help_line).wrap(Wrap { trim: true }),
            chunks[1],
        );
    }
}
