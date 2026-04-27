use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

mod input;
mod keymap;
mod wayland;

#[derive(Parser, Debug)]
#[command(name = "computer", version, about = "Wayland desktop automation for agents")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// List Wayland outputs (monitors)
    Outputs {
        /// Emit JSON instead of human text
        #[arg(long)]
        json: bool,
    },
    /// List toplevel windows via ext-foreign-toplevel-list-v1
    Windows {
        #[arg(long)]
        json: bool,
    },
    /// Capture an output to a PNG file
    Screenshot {
        /// wl_output name to capture (see `outputs`). Defaults to first output.
        #[arg(long)]
        output: Option<String>,
        /// Path to write PNG to. Defaults to stdout.
        #[arg(short = 'o', long = "file")]
        file: Option<String>,
    },
    /// Mouse subcommands
    Mouse {
        #[command(subcommand)]
        action: MouseAction,
    },
    /// Keyboard subcommands
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
    /// Type a literal string of text
    Type {
        text: String,
        /// Inter-keystroke delay in ms
        #[arg(long, default_value_t = 8)]
        delay_ms: u64,
    },
    /// Sleep for milliseconds
    Sleep { ms: u64 },
}

#[derive(Subcommand, Debug)]
enum MouseAction {
    /// Move pointer to absolute pixel position. Requires --output (or first output is used).
    Move {
        x: i32,
        y: i32,
        #[arg(long)]
        output: Option<String>,
    },
    /// Move pointer relative by dx,dy pixels.
    MoveRel { dx: i32, dy: i32 },
    /// Click a button.
    Click {
        #[arg(long, value_enum, default_value_t = Button::Left)]
        button: Button,
        #[arg(long, default_value_t = 1)]
        count: u32,
    },
    /// Press button down (without releasing).
    Down {
        #[arg(long, value_enum, default_value_t = Button::Left)]
        button: Button,
    },
    /// Release a button.
    Up {
        #[arg(long, value_enum, default_value_t = Button::Left)]
        button: Button,
    },
    /// Scroll by (dx,dy). Positive y = scroll down.
    Scroll { dx: i32, dy: i32 },
}

#[derive(Subcommand, Debug)]
enum KeyAction {
    /// Tap a key (press + release).
    Tap { key: String },
    /// Hold a key (press without releasing).
    Down { key: String },
    /// Release a key.
    Up { key: String },
    /// Tap a chord like "ctrl+shift+t".
    Chord { combo: String },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Button {
    Left,
    Right,
    Middle,
    Side,
    Extra,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Outputs { json } => wayland::outputs::run(json),
        Cmd::Windows { json } => wayland::windows::run(json),
        Cmd::Screenshot { output, file } => wayland::screenshot::run(output.as_deref(), file.as_deref()),
        Cmd::Mouse { action } => input::mouse(action),
        Cmd::Key { action } => input::key(action),
        Cmd::Type { text, delay_ms } => input::type_text(&text, delay_ms),
        Cmd::Sleep { ms } => {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(())
        }
    }
}
