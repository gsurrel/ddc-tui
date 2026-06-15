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

use crate::app::{App, FocusedElement};

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

// Add Send + Sync + 'static bounds to B::Error to satisfy anyhow
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B::Error: Send + Sync + 'static, // FIXES THE LIFETIME ERROR
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
                    KeyCode::Char('r') => app.refresh_values(),
                    KeyCode::Up => match app.focused_element {
                        FocusedElement::MonitorList => {
                            if app.selected_monitor_idx > 0 {
                                app.selected_monitor_idx -= 1;
                                app.refresh_values();
                            }
                        }
                        FocusedElement::Brightness => {
                            app.focused_element = FocusedElement::MonitorList
                        }
                        FocusedElement::Contrast => {
                            app.focused_element = FocusedElement::Brightness
                        }
                    },
                    KeyCode::Down => match app.focused_element {
                        FocusedElement::MonitorList => {
                            app.focused_element = FocusedElement::Brightness
                        }
                        FocusedElement::Brightness => {
                            app.focused_element = FocusedElement::Contrast
                        }
                        FocusedElement::Contrast => {}
                    },
                    KeyCode::Left => match app.focused_element {
                        FocusedElement::Brightness => app.adjust_brightness(-5),
                        FocusedElement::Contrast => app.adjust_contrast(-5),
                        _ => {}
                    },
                    KeyCode::Right => match app.focused_element {
                        FocusedElement::Brightness => app.adjust_brightness(5),
                        FocusedElement::Contrast => app.adjust_contrast(5),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
