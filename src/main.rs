//! aform-rs: Terminal Stockholm alignment editor.
//!
//! A vim-style terminal editor for RNA sequence alignments in Stockholm format,
//! inspired by Emacs ralee mode.

mod app;
mod color;
mod editor;
mod external;
mod input;
mod stockholm;
mod structure;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;

/// Terminal Stockholm alignment editor.
#[derive(Parser, Debug)]
#[command(name = "aform-rs")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Stockholm alignment file to open.
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Initial color scheme (none, structure, base, conservation).
    #[arg(short, long, default_value = "none")]
    color: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Set color scheme
    if let Some(scheme) = app::ColorScheme::from_str(&args.color) {
        app.color_scheme = scheme;
    }

    // Load file if provided
    if let Some(path) = args.file {
        if let Err(e) = app.load_file(&path) {
            app.set_status(format!("Error: {}", e));
        }
    }

    // Run main loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        // Calculate visible dimensions for viewport adjustment
        let size = terminal.size()?;
        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
        let (visible_rows, visible_cols) = ui::visible_dimensions(
            area,
            app.alignment.num_sequences(),
            app.alignment.max_id_len(),
            app.show_ruler,
            app.show_row_numbers,
            app.split_mode,
        );

        // Adjust viewport to keep cursor visible
        app.adjust_viewport(visible_rows, visible_cols);

        // Draw UI
        terminal.draw(|f| ui::render(f, app))?;

        // Handle events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                input::handle_key(app, key, visible_rows);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
