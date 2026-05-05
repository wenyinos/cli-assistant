//! Markdown to ANSI terminal rendering.
//!
//! Converts markdown text into ANSI-formatted terminal output with colored
//! headers, code blocks, lists, links, and inline formatting.

use super::colors::{colorize, stylize, Style};
use super::theme::Theme;

/// Convert markdown text to ANSI-formatted terminal output.
pub fn markdown_to_ansi(text: &str, theme: &Theme, plain: bool) -> String {
    if plain {
        return text.to_string();
    }

    let mut output = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        // Fenced code blocks
        if line.trim_start().starts_with("```") {
            if in_code_block {
                // End of code block
                output.push_str(&render_code_block(&code_block_lines, &code_block_lang, theme));
                code_block_lines.clear();
                code_block_lang.clear();
                in_code_block = false;
            } else {
                // Start of code block
                let lang = line.trim_start().trim_start_matches('`').trim().to_string();
                code_block_lang = lang;
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block_lines.push(line.to_string());
            continue;
        }

        // Headers
        if line.starts_with("# ") {
            output.push_str(&colorize(&format!("\n{}\n", &line[2..]), theme.header));
            continue;
        }
        if line.starts_with("## ") {
            output.push_str(&colorize(&format!("\n{}\n", &line[3..]), theme.header));
            continue;
        }
        if line.starts_with("### ") {
            output.push_str(&colorize(&format!("\n{}\n", &line[4..]), theme.header));
            continue;
        }

        // Horizontal rule
        if line.trim() == "---" || line.trim() == "***" || line.trim() == "___" {
            output.push_str(&colorize(&"─".repeat(60), theme.horizontal_rule));
            output.push('\n');
            continue;
        }

        // Unordered list
        if line.starts_with("- ") || line.starts_with("* ") {
            output.push_str(&format!("• {}\n", render_inline(&line[2..], theme)));
            continue;
        }

        // Ordered list
        if let Some(rest) = strip_ordered_list_prefix(line) {
            output.push_str(&format!("  {}", render_inline(rest, theme)));
            output.push('\n');
            continue;
        }

        // Blockquote
        if line.starts_with("> ") {
            output.push_str(&format!("│ {}\n", render_inline(&line[2..], theme)));
            continue;
        }

        // Regular paragraph
        output.push_str(&render_inline(line, theme));
        output.push('\n');
    }

    // Handle unclosed code block
    if in_code_block && !code_block_lines.is_empty() {
        output.push_str(&render_code_block(&code_block_lines, &code_block_lang, theme));
    }

    output
}

/// Render inline markdown elements (bold, italic, code, links).
fn render_inline(text: &str, theme: &Theme) -> String {
    let mut result = text.to_string();

    // Inline code: `code`
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start + 1..].find('`') {
            let code = &result[start + 1..start + 1 + end];
            let replacement = colorize(code, theme.inline_code);
            result = format!("{}{}{}", &result[..start], replacement, &result[start + 2 + end..]);
        } else {
            break;
        }
    }

    // Bold: **text** or __text__
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let inner = &result[start + 2..start + 2 + end];
            let replacement = stylize(inner, Style::Bold);
            result = format!("{}{}{}", &result[..start], replacement, &result[start + 4 + end..]);
        } else {
            break;
        }
    }

    // Italic: *text* or _text_
    while let Some(start) = result.find('*') {
        if result.get(start + 1..start + 2) == Some("*") {
            continue; // Skip ** (bold)
        }
        if let Some(end) = result[start + 1..].find('*') {
            let inner = &result[start + 1..start + 1 + end];
            let replacement = stylize(inner, Style::Italic);
            result = format!("{}{}{}", &result[..start], replacement, &result[start + 2 + end..]);
        } else {
            break;
        }
    }

    // Links: [text](url)
    while let Some(start) = result.find('[') {
        if let Some(mid) = result[start..].find("](") {
            if let Some(end) = result[start + mid + 2..].find(')') {
                let text = &result[start + 1..start + mid];
                let url = &result[start + mid + 2..start + mid + 2 + end];
                let replacement = format!(
                    "{} ({})",
                    colorize(text, theme.link),
                    colorize(url, theme.link)
                );
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    replacement,
                    &result[start + mid + 3 + end..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    result
}

/// Render a code block with a border.
fn render_code_block(lines: &[String], lang: &str, theme: &Theme) -> String {
    let max_width = lines.iter().map(|l| l.len()).max().unwrap_or(0).max(40);
    let mut output = String::new();

    // Header border
    if lang.is_empty() {
        output.push_str(&colorize(&format!("┌{}┐", "─".repeat(max_width + 2)), theme.code_block_border));
    } else {
        let lang_label = format!(" {} ", lang);
        let remaining = max_width.saturating_sub(lang_label.len()) + 2;
        output.push_str(&colorize("┌", theme.code_block_border));
        output.push_str(&colorize(&lang_label, theme.header));
        output.push_str(&colorize(&"─".repeat(remaining), theme.code_block_border));
        output.push_str(&colorize("┐", theme.code_block_border));
    }
    output.push('\n');

    // Code lines
    for line in lines {
        let padding = max_width.saturating_sub(line.len());
        output.push_str(&colorize("│ ", theme.code_block_border));
        output.push_str(&colorize(line, theme.code_block_line));
        output.push_str(&" ".repeat(padding));
        output.push_str(&colorize(" │", theme.code_block_border));
        output.push('\n');
    }

    // Footer border
    output.push_str(&colorize(&format!("└{}┘", "─".repeat(max_width + 2)), theme.code_block_border));
    output.push('\n');

    output
}

/// Try to strip an ordered list prefix like "1. " from a line.
fn strip_ordered_list_prefix(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let mut chars = trimmed.chars();
    // Consume digits
    while chars.next()?.is_ascii_digit() {}
    // Expect ". "
    if chars.next()? != '.' {
        return None;
    }
    if chars.next()? != ' ' {
        return None;
    }
    Some(&trimmed[trimmed.len() - chars.as_str().len()..])
}
