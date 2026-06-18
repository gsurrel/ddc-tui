use crate::app::{App, FeatureType, FocusArea, UIMode, VcpFeatureInfo};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < 40 || area.height < 10 {
        frame.render_widget(
            Paragraph::new("Terminal too small.").block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(4),
        ])
        .split(area);

    draw_title(frame, main_chunks[0]);
    draw_body(frame, main_chunks[1], app);
    draw_status_controls(frame, main_chunks[2], app);

    if app.is_probing {
        draw_probing_overlay(frame, area);
    }
    if app.ui_mode == UIMode::ProfileSearch {
        draw_search_overlay(frame, area, app);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_probing_overlay(frame: &mut Frame, area: Rect) {
    let inner = centered_rect(50, 20, area);
    frame.render_widget(Clear, inner);
    let text = Paragraph::new("Reading VCP features...\nPlease wait.")
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Probing Monitor...")
                .style(Style::default().bg(Color::DarkGray).fg(Color::White)),
        );
    frame.render_widget(text, inner);
}

fn draw_search_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let inner = centered_rect(60, 60, area);
    frame.render_widget(Clear, inner);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Profile Override (Esc: cancel, Enter: select)")
        .style(Style::default().bg(Color::DarkGray));

    let inner_area = block.inner(inner);
    frame.render_widget(block, inner);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner_area);

    frame.render_widget(
        Paragraph::new(app.search_query.as_str())
            .style(Style::default().fg(Color::Yellow).bg(Color::Black))
            .block(Block::default().borders(Borders::ALL).title("Search Query")),
        chunks[0],
    );

    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, &profile_idx)| {
            let p = &crate::db::MONITOR_PROFILES[profile_idx];
            let style = if i == app.search_selected_idx {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            };
            ListItem::new(format!("{} ({})", p.display_name, p.pnp_name)).style(style)
        })
        .collect();

    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("Results")),
        chunks[1],
    );
}

fn draw_title(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "DDC/CI Monitor Control",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]))
        .block(Block::default().borders(Borders::ALL)),
        area,
    );
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
                spans.push(Span::raw("  "));
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

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Monitors", title_style)),
        ),
        area,
    );
}

fn draw_monitor_controls(frame: &mut Frame, area: Rect, app: &mut App) {
    let monitor_idx = app.selected_monitor_idx;
    let (monitor_name, profile_chain, features) = match app.monitors.get(monitor_idx) {
        Some(m) => (m.name.clone(), m.profile_chain.clone(), m.features.clone()),
        None => ("<no monitor>".to_string(), Vec::new(), Vec::new()),
    };

    let chain_str = profile_chain.join(" -> ");
    let block_title = if chain_str.is_empty() {
        format!("Selected: {}", monitor_name)
    } else {
        format!("Selected: {} ({})", monitor_name, chain_str)
    };

    let outer = Block::default().borders(Borders::ALL).title(block_title);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if inner.width < 10 || inner.height < 3 {
        frame.render_widget(
            Paragraph::new("Not enough space").block(Block::default()),
            inner,
        );
        return;
    }

    draw_compact_features(frame, inner, app, &features);
}

fn render_pills<'a>(
    options: &'a [String],
    current_idx: usize,
    selected_idx: usize,
    is_focused: bool,
) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, opt) in options.iter().enumerate() {
        let style = if is_focused && i == selected_idx {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if i == current_idx {
            Style::default()
                .fg(Color::Green)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {} ", opt), style));
        spans.push(Span::raw(" ")); // Space allows native word-wrapping
    }
    Line::from(spans)
}

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
    let total_features = features.len();

    let half_screen = max_rows / 2;
    if app.selected_vcp_idx < app.scroll_offset + half_screen {
        app.scroll_offset = app.selected_vcp_idx.saturating_sub(half_screen);
    } else if app.selected_vcp_idx >= app.scroll_offset + max_rows - half_screen {
        app.scroll_offset = (app.selected_vcp_idx + half_screen + 1).saturating_sub(max_rows);
    }
    let max_scroll = total_features.saturating_sub(max_rows);
    app.scroll_offset = app.scroll_offset.min(max_scroll);

    let start_idx = app.scroll_offset;
    let end_idx = (start_idx + max_rows).min(total_features);
    let features_to_draw = &features[start_idx..end_idx];

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
        let global_idx = start_idx + i;
        let is_focused =
            app.focus_area == FocusArea::VcpFeatures && global_idx == app.selected_vcp_idx;

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(row);

        let label_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(feature.name.clone()).style(label_style),
            cols[0],
        );

        match &feature.feature_type {
            FeatureType::Continuous { max } => {
                let ratio = if *max == 0 {
                    0.0
                } else {
                    (feature.current as f64 / *max as f64).clamp(0.0, 1.0)
                };
                let gauge = Gauge::default()
                    .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
                    .ratio(ratio);
                frame.render_widget(gauge, cols[1]);
            }
            FeatureType::Discrete { options, values } => {
                let current_idx = values
                    .iter()
                    .position(|&v| v == feature.current)
                    .unwrap_or(0);
                let pills_line =
                    render_pills(options, current_idx, app.selected_pill_idx, is_focused);
                let p = Paragraph::new(pills_line).wrap(Wrap { trim: true });
                frame.render_widget(p, cols[1]);
            }
            FeatureType::ActionGroup { actions } => {
                let options: Vec<String> = actions.iter().map(|a| a.name.clone()).collect();
                let pills_line = render_pills(&options, 0, app.selected_pill_idx, is_focused);
                let p = Paragraph::new(pills_line).wrap(Wrap { trim: true });
                frame.render_widget(p, cols[1]);
            }
        }
    }

    if total_features > max_rows {
        let scroll_text = format!("[{}/{}]", app.scroll_offset + 1, total_features);
        let scroll_area = Rect::new(
            area.x + area.width - scroll_text.len() as u16 - 1,
            area.y,
            scroll_text.len() as u16,
            1,
        );
        frame.render_widget(
            Paragraph::new(scroll_text).style(Style::default().fg(Color::DarkGray)),
            scroll_area,
        );
    }
}

fn draw_status_controls(frame: &mut Frame, area: Rect, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .title("Status & Controls");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if inner.width < 10 || inner.height < 1 {
        return;
    }

    let mut constraints = vec![Constraint::Length(1)];
    if inner.height > 1 {
        constraints.push(Constraint::Min(1));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let focus_text = match app.focus_area {
        FocusArea::MonitorList => "Focus: Monitors",
        FocusArea::VcpFeatures => "Focus: Controls",
    };

    let display_status = if app.is_probing {
        "⏳ Probing monitor...".to_string()
    } else {
        app.status_message.clone()
    };

    let status_line = if inner.width > 40 {
        let mut s = display_status;
        let max_status_len = inner.width as usize - focus_text.len() - 3;
        if s.len() > max_status_len && max_status_len > 0 {
            s.truncate(max_status_len - 1);
            s.push('…');
        }
        Line::from(vec![
            Span::styled(
                s,
                if app.is_error || app.is_probing {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ),
            Span::raw("   "),
            Span::styled(
                focus_text,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![Span::styled(
            display_status,
            if app.is_error || app.is_probing {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            },
        )])
    };
    frame.render_widget(Paragraph::new(status_line), chunks[0]);

    if chunks.len() > 1 {
        let mut help_spans = vec![
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Nav "),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" Focus "),
            Span::styled("p", Style::default().fg(Color::Yellow)),
            Span::raw(" Profile "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit"),
        ];

        if app.focus_area == FocusArea::VcpFeatures {
            if let Some(m) = app.monitors.get(app.selected_monitor_idx) {
                if let Some(f) = m.features.get(app.selected_vcp_idx) {
                    let mut contextual = vec![
                        Span::styled("←/→", Style::default().fg(Color::Yellow)),
                        Span::raw(" "),
                    ];
                    match &f.feature_type {
                        FeatureType::Continuous { .. } => {
                            contextual.push(Span::raw("Adj (Shift: faster) "));
                        }
                        FeatureType::Discrete { .. } | FeatureType::ActionGroup { .. } => {
                            contextual.push(Span::raw("Select "));
                            contextual
                                .push(Span::styled("Enter", Style::default().fg(Color::Yellow)));
                            contextual.push(Span::raw(" Apply "));
                        }
                    }

                    let mut new_help = vec![
                        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                        Span::raw(" Nav "),
                    ];
                    new_help.extend(contextual);
                    new_help.extend(vec![
                        Span::styled("Tab", Style::default().fg(Color::Yellow)),
                        Span::raw(" Focus "),
                        Span::styled("p", Style::default().fg(Color::Yellow)),
                        Span::raw(" Profile "),
                        Span::styled("r", Style::default().fg(Color::Yellow)),
                        Span::raw(" Refresh "),
                        Span::styled("q", Style::default().fg(Color::Yellow)),
                        Span::raw(" Quit"),
                    ]);
                    help_spans = new_help;
                }
            }
        }

        let help_line = Line::from(help_spans);
        frame.render_widget(
            Paragraph::new(help_line).wrap(Wrap { trim: true }),
            chunks[1],
        );
    }
}
