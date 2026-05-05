//! Theme configuration for the renderer.

use super::colors::Color;

/// Color theme for terminal rendering.
#[derive(Debug, Clone)]
pub struct Theme {
    // General output colors
    #[allow(dead_code)]
    pub info: Color,
    pub warning: Color,
    pub notice: Color,
    pub error: Color,

    // Markdown formatting colors
    pub inline_code: Color,
    pub code_block_line: Color,
    pub code_block_border: Color,
    pub header: Color,
    pub link: Color,
    pub horizontal_rule: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            info: Color::BrightBlue,
            warning: Color::Yellow,
            notice: Color::BrightYellow,
            error: Color::Red,
            inline_code: Color::Cyan,
            code_block_line: Color::Cyan,
            code_block_border: Color::BrightRed,
            header: Color::Green,
            link: Color::BrightBlue,
            horizontal_rule: Color::BrightBlack,
        }
    }
}
