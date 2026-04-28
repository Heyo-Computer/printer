use clap::{Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum MouseAction {
    /// Move pointer to absolute position. On Linux this is pixels on a chosen
    /// output; on macOS it's points in the global display coordinate space.
    Move {
        x: i32,
        y: i32,
        #[arg(long)]
        output: Option<String>,
    },
    /// Move pointer relative by dx,dy.
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
pub enum KeyAction {
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
pub enum Button {
    Left,
    Right,
    Middle,
    Side,
    Extra,
}
