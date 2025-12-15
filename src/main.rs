//! aform-rs: Terminal Stockholm alignment editor.
//!
//! A vim-style terminal editor for RNA sequence alignments in Stockholm format,
//! inspired by Emacs ralee mode.

mod app;
mod clustering;
mod color;
mod editor;
mod history;
mod input;
mod stockholm;
mod structure;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

use app::{App, TerminalTheme};

/// Terminal Stockholm alignment editor.
#[derive(Parser, Debug)]
#[command(name = "aform-rs")]
#[command(author, version, about, long_about = None)]
#[command(after_help = AFTER_HELP)]
struct Args {
    /// Stockholm alignment file to open.
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Initial color scheme (none, structure, base, conservation, compensatory).
    #[arg(short, long, default_value = "none")]
    color: String,

    /// Show consensus sequence.
    #[arg(long)]
    consensus: bool,

    /// Show conservation bar.
    #[arg(long)]
    conservation: bool,

    /// Cluster sequences by similarity.
    #[arg(long)]
    cluster: bool,

    /// Collapse identical sequences.
    #[arg(long)]
    collapse: bool,

    /// Show dendrogram tree (implies --cluster).
    #[arg(long)]
    tree: bool,

    /// Show column ruler.
    #[arg(long)]
    ruler: bool,

    /// Show row numbers.
    #[arg(long)]
    rownum: bool,

    /// Show short IDs (strip coordinate suffix like /10000-20000).
    #[arg(long)]
    shortid: bool,
}

const AFTER_HELP: &str = "\
INTERACTIVE COMMANDS:
  Press ':' to enter command mode, then type a command and press Enter.
  Press '?' for interactive help overlay.

VISUALIZATION:
  :ruler          Toggle column ruler
  :rownum         Toggle row numbers
  :shortid        Toggle short IDs (strip /start-end suffix)
  :split / :sp    Horizontal split view
  :vsplit / :vs   Vertical split view
  :only           Close split view
  :tree           Toggle dendrogram tree (requires :cluster)

CONSERVATION:
  :conservation   Toggle conservation bar (shows column-wise identity)
  :consbar        Alias for :conservation

CONSENSUS:
  :consensus      Toggle consensus sequence display

CLUSTERING:
  :cluster        Cluster sequences by similarity (UPGMA)
  :uncluster      Restore original sequence order
  :collapse       Toggle collapsing identical sequences
  :tree           Show/hide dendrogram tree

COLOR SCHEMES:
  :color none         No coloring
  :color structure    Color by secondary structure (helix pairs)
  :color base         Color by nucleotide/amino acid identity
  :color conservation Color by column conservation
  :color compensatory Color by compensatory mutations (requires SS_cons)

  Aliases: ss=structure, nt/residue/aa/protein=base, cons=conservation, comp=compensatory
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Detect terminal theme before entering raw mode
    let terminal_theme = detect_terminal_theme();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    app.terminal_theme = terminal_theme;

    // Set color scheme
    if let Some(scheme) = app::ColorScheme::from_str(&args.color) {
        app.color_scheme = scheme;
    }

    // Load file if provided
    if let Some(path) = args.file {
        if let Err(e) = app.load_file(&path) {
            app.set_status(format!("Error: {}", e));
        } else {
            // Auto-detect sequence type and precompute collapse groups
            app.detect_sequence_type();
            app.precompute_collapse_groups();
        }
    }

    // Apply display options from CLI (only enable, don't disable defaults)
    if args.consensus {
        app.show_consensus = true;
    }
    if args.conservation {
        app.show_conservation_bar = true;
    }
    if args.ruler {
        app.show_ruler = true;
    }
    if args.rownum {
        app.show_row_numbers = true;
    }
    if args.shortid {
        app.show_short_ids = true;
    }

    // Apply clustering options (only if file loaded)
    if app.alignment.num_sequences() > 0 {
        if args.collapse {
            app.toggle_collapse_identical();
        }
        if args.cluster || args.tree {
            app.cluster_sequences();
        }
        if args.tree {
            app.show_tree = true;
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

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        // Calculate visible dimensions for viewport adjustment
        let size = terminal.size()?;
        let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
        let tree_display_width = if app.show_tree && app.cluster_tree.is_some() {
            app.tree_width + 1
        } else {
            0
        };
        let max_id_len = if app.show_short_ids {
            app.alignment.max_short_id_len()
        } else {
            app.alignment.max_id_len()
        };
        let (visible_rows, visible_cols) = ui::visible_dimensions(
            area,
            app.visible_sequence_count(),
            max_id_len,
            app.show_ruler,
            app.show_row_numbers,
            app.show_short_ids,
            app.split_mode,
            app.alignment.ss_cons().is_some(),
            app.alignment.rf().is_some(),
            app.show_rf_bar,
            app.alignment.pp_cons().is_some(),
            app.show_pp_cons,
            app.show_consensus,
            app.show_conservation_bar,
            app.max_collapse_count(),
            tree_display_width,
            app.alignment.width(),
        );

        // Adjust viewport to keep cursor visible
        app.adjust_viewport(visible_rows, visible_cols);

        // Draw UI
        terminal.draw(|f| ui::render(f, app))?;

        // Handle events
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            input::handle_key(app, key, visible_rows);
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Detect terminal background theme using termbg.
fn detect_terminal_theme() -> TerminalTheme {
    // termbg needs a timeout for terminals that don't respond
    let timeout = std::time::Duration::from_millis(100);

    match termbg::theme(timeout) {
        Ok(termbg::Theme::Light) => TerminalTheme::Light,
        Ok(termbg::Theme::Dark) => TerminalTheme::Dark,
        Err(_) => TerminalTheme::Dark, // Default to dark on detection failure
    }
}
