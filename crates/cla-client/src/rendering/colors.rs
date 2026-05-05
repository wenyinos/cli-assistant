//! ANSI color and style utilities.

/// ANSI color codes for terminal output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Color {
    Normal,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    /// ANSI escape code for this color.
    pub fn code(&self) -> &'static str {
        match self {
            Color::Normal => "\x1b[0m",
            Color::Black => "\x1b[30m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
            Color::BrightBlack => "\x1b[90m",
            Color::BrightRed => "\x1b[91m",
            Color::BrightGreen => "\x1b[92m",
            Color::BrightYellow => "\x1b[93m",
            Color::BrightBlue => "\x1b[94m",
            Color::BrightMagenta => "\x1b[95m",
            Color::BrightCyan => "\x1b[96m",
            Color::BrightWhite => "\x1b[97m",
        }
    }
}

/// ANSI style codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Style {
    Normal,
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

impl Style {
    pub fn code(&self) -> &'static str {
        match self {
            Style::Normal => "\x1b[0m",
            Style::Bold => "\x1b[1m",
            Style::Italic => "\x1b[3m",
            Style::Underline => "\x1b[4m",
            Style::Strikethrough => "\x1b[9m",
        }
    }
}

/// Wrap text with ANSI color codes.
pub fn colorize(text: &str, color: Color) -> String {
    if std::env::var("NO_COLOR").is_ok() {
        return text.to_string();
    }
    format!("{}{}\x1b[0m", color.code(), text)
}

/// Wrap text with ANSI style codes.
pub fn stylize(text: &str, style: Style) -> String {
    if std::env::var("NO_COLOR").is_ok() {
        return text.to_string();
    }
    format!("{}{}\x1b[0m", style.code(), text)
}
