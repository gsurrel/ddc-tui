mod app;
mod ddc;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

use crate::app::{App, FocusArea};

#[derive(Parser, Debug)]
#[command(name = "ddc-tui")]
#[command(about = "Cross-platform DDC/CI Monitor Control TUI", long_about = None)]
struct Args {
    /// List available monitors and exit
    #[arg(short, long)]
    list: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.list {
        let ddc = ddc::DdcController::new()?;
        let monitors = ddc.get_monitors()?;
        if monitors.is_empty() {
            println!("No DDC/CI compatible monitors found.");
        } else {
            println!("Available Monitors:");
            for (i, m) in monitors.iter().enumerate() {
                println!("  [{}] {}", i, m.name);
            }
        }
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    while app.running {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => app.running = false,
                    KeyCode::Char('r') => app.refresh_features(),
                    KeyCode::Tab => {
                        app.focus_area = match app.focus_area {
                            FocusArea::MonitorList => FocusArea::VcpFeatures,
                            FocusArea::VcpFeatures => FocusArea::MonitorList,
                        };
                    }
                    KeyCode::Up => match app.focus_area {
                        FocusArea::MonitorList => {
                            if app.selected_monitor_idx > 0 {
                                app.selected_monitor_idx -= 1;
                                app.selected_vcp_idx = 0;
                                app.refresh_features();
                            }
                        }
                        FocusArea::VcpFeatures => {
                            if app.selected_vcp_idx > 0 {
                                app.selected_vcp_idx -= 1;
                            }
                        }
                    },
                    KeyCode::Down => match app.focus_area {
                        FocusArea::MonitorList => {
                            if app.selected_monitor_idx + 1 < app.monitors.len() {
                                app.selected_monitor_idx += 1;
                                app.selected_vcp_idx = 0;
                                app.refresh_features();
                            }
                        }
                        FocusArea::VcpFeatures => {
                            let max_idx = app.monitors[app.selected_monitor_idx].features.len();
                            if app.selected_vcp_idx + 1 < max_idx {
                                app.selected_vcp_idx += 1;
                            }
                        }
                    },
                    KeyCode::Left => {
                        if app.focus_area == FocusArea::VcpFeatures {
                            app.adjust_selected_feature(-1);
                        }
                    }
                    KeyCode::Right => {
                        if app.focus_area == FocusArea::VcpFeatures {
                            app.adjust_selected_feature(1);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
