pub const ESCAPE_SEQUENCE_CLEAR_SCREEN: &str = "\x1b[2J";
pub const ESCAPE_SEQUENCE_CLEAR_LINE: &str = "\x1b[K";
pub const ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION: &str = "\x1b[H";
pub const ESCAPE_SEQUENCE_STYLE_RESET: &str = "\x1b[m";
pub const ESCAPE_SEQUENCE_STYLE_REVERSE: &str = "\x1b[7m";
pub const ESCAPE_SEQUENCE_HIDE_CURSOR: &str = "\x1b[?25l";
pub const ESCAPE_SEQUENCE_SHOW_CURSOR: &str = "\x1b[?25h";

pub fn move_cursor(x: usize, y: usize) -> String {
    format!("\x1b[{};{}H", y + 1, x + 1)
}

pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
}

impl Color {
    pub fn foreground_escape_sequence(&self) -> &'static str {
        match self {
            Color::Black => "\x1b[30m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
            Color::Default => "\x1b[39m",
        }
    }

    pub fn background_escape_sequence(&self) -> &'static str {
        match self {
            Color::Black => "\x1b[40m",
            Color::Red => "\x1b[41m",
            Color::Green => "\x1b[42m",
            Color::Yellow => "\x1b[43m",
            Color::Blue => "\x1b[44m",
            Color::Magenta => "\x1b[45m",
            Color::Cyan => "\x1b[46m",
            Color::White => "\x1b[47m",
            Color::Default => "\x1b[49m",
        }
    }
}
