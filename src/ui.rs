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
            "Terminal too small. Resize to use the UI.",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]))
        .block(Block::default().borders(Borders::ALL).title("DDC/CI"));
        frame.render_widget(p, area);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(6),    // Body (left + right)
            Constraint::Length(4), // Merged Status Controls (taller to fit both)
        ])
        .split(area);

    draw_title(frame, main_chunks[0]);
    draw_body(frame, main_chunks[1], app);
    draw_status_controls(frame, main_chunks[2], app);
}

/// Title block
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

/// Body: left column = monitors, right column = selected monitor controls
fn draw_body(frame: &mut Frame, area: Rect, app: &mut App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(10)])
        .split(area);

    draw_monitor_list(frame, cols[0], app);
    draw_monitor_controls(frame, cols[1], app);
}

/// Monitor list with focus highlight
fn draw_monitor_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let mut spans = Vec::new();
            spans.push(Span::raw(m.name.clone()));
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

    let title_style = if app.focus_area == FocusArea::MonitorList {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Monitors", title_style)),
    );

    frame.render_widget(list, area);
}

/// Right column: selected monitor name + compact label:gauge rows
fn draw_monitor_controls(frame: &mut Frame, area: Rect, app: &mut App) {
    // Snapshot minimal data (no long-lived borrow of app)
    let monitor_idx = app.selected_monitor_idx;
    let (monitor_name, features): (String, Vec<VcpFeatureInfo>) =
        match app.monitors.get(monitor_idx) {
            Some(m) => (m.name.clone(), m.features.clone()),
            None => ("<no monitor>".to_string(), Vec::new()),
        };

    let outer = Block::default()
        .borders(Borders::ALL)
        .title("Selected Monitor");
    frame.render_widget(outer, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner.width < 10 || inner.height < 3 {
        let p = Paragraph::new(Line::from(vec![Span::raw("Not enough space for controls")]))
            .block(Block::default());
        frame.render_widget(p, inner);
        return;
    }

    let header_style = Style::default()
        .fg(if app.focus_area == FocusArea::MonitorList {
            Color::White
        } else {
            Color::Cyan
        })
        .add_modifier(Modifier::BOLD);

    let header = Paragraph::new(Line::from(vec![Span::styled(
        monitor_name.clone(),
        header_style,
    )]))
    .wrap(Wrap { trim: true });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(header, chunks[0]);

    draw_compact_features(frame, chunks[1], app, &features);
}

/// Compact label:gauge layout for features with safe ratio and fallbacks
fn draw_compact_features(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
    features: &[VcpFeatureInfo],
) {
    if features.is_empty() {
        let p = Paragraph::new(Line::from(vec![Span::raw(
            "No supported VCP features found for this monitor.",
        )]))
        .block(Block::default());
        frame.render_widget(p, area);
        return;
    }

    let max_rows = area.height as usize;
    if max_rows == 0 {
        return;
    }

    let features_to_draw = &features[..features.len().min(max_rows)];

    let constraints: Vec<Constraint> = features_to_draw
        .iter()
        .map(|_| Constraint::Length(1))
        .collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, feature) in features_to_draw.iter().enumerate() {
        let row = rows[i];

        if row.width < 10 {
            let fallback = Paragraph::new(Line::from(vec![Span::raw(format!(
                "{}: {}/{}",
                feature.name, feature.current, feature.max
            ))]));
            frame.render_widget(fallback, row);
            continue;
        }

        let mut label_width = (row.width as f32 * 0.35).max(12.0) as u16;
        if label_width + 6 >= row.width {
            label_width = row.width.saturating_sub(6);
        }
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(label_width), Constraint::Min(10)])
            .split(row);

        let label_text = format!(
            "{} (0x{:02X}): {}/{}",
            feature.name, feature.code, feature.current, feature.max
        );
        let is_focused = app.focus_area == FocusArea::VcpFeatures && i == app.selected_vcp_idx;

        let label_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let label = Paragraph::new(Line::from(vec![Span::styled(label_text, label_style)]));
        frame.render_widget(label, cols[0]);

        let ratio = if feature.max == 0 {
            0.0
        } else {
            let raw = (feature.current as f64) / (feature.max as f64);
            if raw.is_finite() {
                raw.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };

        if cols[1].width < 6 {
            let fallback = Paragraph::new(Line::from(vec![Span::raw(format!(
                "{}/{}",
                feature.current, feature.max
            ))]));
            frame.render_widget(fallback, cols[1]);
            continue;
        }

        let gauge_style = Style::default().fg(if is_focused {
            Color::Yellow
        } else {
            Color::Green
        });

        let gauge = Gauge::default()
            .gauge_style(gauge_style.add_modifier(if is_focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }))
            .ratio(ratio)
            .label(format!("{}/{}", feature.current, feature.max));

        frame.render_widget(gauge, cols[1]);
    }
}

/// Merged Status Controls block
fn draw_status_controls(frame: &mut Frame, area: Rect, app: &App) {
    // Single bordered block containing status, focus indicator, and controls/help
    let outer = Block::default()
        .borders(Borders::ALL)
        .title("Status Controls");
    frame.render_widget(outer, area);

    // inner padded area
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner.width < 10 || inner.height < 1 {
        return;
    }

    // Split vertically: status line on top, help lines below
    // Give status one line, help the rest
    let mut constraints = vec![Constraint::Length(1)];
    if inner.height > 1 {
        constraints.push(Constraint::Min(1));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // Status line: includes status message and a small Focus indicator on the right
    let focus_text = match app.focus_area {
        FocusArea::MonitorList => "Focus: Monitors",
        FocusArea::VcpFeatures => "Focus: Controls",
    };

    // Build status line with right-aligned focus indicator if space allows
    let status_line = if inner.width > 40 {
        // pad status message and append focus on the right
        let mut s = app.status_message.clone();
        // ensure we don't overflow: truncate if necessary
        let max_status_len = inner.width as usize - focus_text.len() - 3;
        if s.len() > max_status_len && max_status_len > 0 {
            s.truncate(max_status_len - 1);
            s.push('…');
        }
        Line::from(vec![
            Span::styled(
                s,
                if app.is_error {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ),
            Span::raw(" "),
            Span::styled(
                focus_text,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        // narrow: show status only
        Line::from(vec![Span::styled(
            app.status_message.clone(),
            if app.is_error {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        )])
    };

    let status_para = Paragraph::new(status_line);
    frame.render_widget(status_para, chunks[0]);

    // Help area: compact help text, wrap if needed
    if chunks.len() > 1 {
        let help_line = Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate  "),
            Span::styled("←/→", Style::default().fg(Color::Yellow)),
            Span::raw(" Adjust  "),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" Switch Focus  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit"),
        ]);

        let help = Paragraph::new(help_line).wrap(Wrap { trim: true });
        frame.render_widget(help, chunks[1]);
    }
}
