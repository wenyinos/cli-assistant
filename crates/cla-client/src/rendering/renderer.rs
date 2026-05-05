//! Renderer for terminal output with color and markdown support.

use super::colors::colorize;
use super::markdown::markdown_to_ansi;
use super::theme::Theme;

/// Terminal renderer that supports colored and markdown output.
pub struct Renderer {
    plain: bool,
    theme: Theme,
}

impl Renderer {
    /// Create a new renderer.
    pub fn new(plain: bool) -> Self {
        Self {
            plain,
            theme: Theme::default(),
        }
    }

    /// Print a normal message.
    pub fn normal(&self, message: &str) {
        println!("{}", message);
    }

    /// Print a warning message (yellow with emoji).
    pub fn warning(&self, message: &str) {
        if self.plain {
            eprintln!("{}", message);
        } else {
            eprintln!("🤔 {}", colorize(message, self.theme.warning));
        }
    }

    /// Print an error message (red with emoji).
    pub fn error(&self, message: &str) {
        if self.plain {
            eprintln!("{}", message);
        } else {
            eprintln!("🙁 {}", colorize(message, self.theme.error));
        }
    }

    /// Print a notice message.
    pub fn notice(&self, message: &str) {
        if self.plain {
            println!("{}", message);
        } else {
            println!("{}", colorize(message, self.theme.notice));
        }
    }

    /// Print an info message.
    #[allow(dead_code)]
    pub fn info(&self, message: &str) {
        if self.plain {
            println!("{}", message);
        } else {
            println!("{}", colorize(message, self.theme.info));
        }
    }

    /// Render markdown text.
    pub fn markdown(&self, text: &str) {
        let rendered = markdown_to_ansi(text, &self.theme, self.plain);
        print!("{}", rendered);
    }
}
