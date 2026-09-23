//! Markdown to ANSI terminal rendering.
//!
//! Converts markdown text into ANSI-formatted terminal output with colored
//! headers, code blocks, lists, links, and inline formatting.

use ratatui::style::{Color, Modifier, Style as TuiStyle};
use ratatui::text::{Line, Span};

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
                output.push_str(&render_code_block(
                    &code_block_lines,
                    &code_block_lang,
                    theme,
                ));
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
        if let Some(stripped) = line.strip_prefix("# ") {
            output.push_str(&colorize(&format!("\n{}\n", stripped), theme.header));
            continue;
        }
        if let Some(stripped) = line.strip_prefix("## ") {
            output.push_str(&colorize(&format!("\n{}\n", stripped), theme.header));
            continue;
        }
        if let Some(stripped) = line.strip_prefix("### ") {
            output.push_str(&colorize(&format!("\n{}\n", stripped), theme.header));
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
        if let Some(stripped) = line.strip_prefix("> ") {
            output.push_str(&format!("│ {}\n", render_inline(stripped, theme)));
            continue;
        }

        // Regular paragraph
        output.push_str(&render_inline(line, theme));
        output.push('\n');
    }

    // Handle unclosed code block
    if in_code_block && !code_block_lines.is_empty() {
        output.push_str(&render_code_block(
            &code_block_lines,
            &code_block_lang,
            theme,
        ));
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
            result = format!(
                "{}{}{}",
                &result[..start],
                replacement,
                &result[start + 2 + end..]
            );
        } else {
            break;
        }
    }

    // Bold: **text** or __text__
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let inner = &result[start + 2..start + 2 + end];
            let replacement = stylize(inner, Style::Bold);
            result = format!(
                "{}{}{}",
                &result[..start],
                replacement,
                &result[start + 4 + end..]
            );
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
            result = format!(
                "{}{}{}",
                &result[..start],
                replacement,
                &result[start + 2 + end..]
            );
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
        output.push_str(&colorize(
            &format!("┌{}┐", "─".repeat(max_width + 2)),
            theme.code_block_border,
        ));
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
    output.push_str(&colorize(
        &format!("└{}┘", "─".repeat(max_width + 2)),
        theme.code_block_border,
    ));
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

// ---------------------------------------------------------------------------
// TUI rendering
// ---------------------------------------------------------------------------

/// Convert markdown text into ratatui lines for the TUI conversation view.
///
/// Handles the constructs the assistant commonly emits — headers, bullets,
/// fenced code blocks, and inline `**bold**` / `` `code` `` — leaving line
/// wrapping to the widget. The ANSI renderer above cannot be reused here:
/// ratatui builds its own cell grid, so styles must become spans.
pub fn markdown_to_lines(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for raw in text.lines() {
        let trimmed = raw.trim_end();

        // Fenced code blocks: drop the fence lines, keep the content indented.
        if trimmed.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("  {}", trimmed.trim_start()),
                TuiStyle::default().fg(Color::Yellow),
            )));
            continue;
        }

        let (content, style, prefix) = if let Some(rest) = trimmed
            .strip_prefix("### ")
            .or_else(|| trimmed.strip_prefix("## "))
            .or_else(|| trimmed.strip_prefix("# "))
        {
            (rest, TuiStyle::default().add_modifier(Modifier::BOLD), "")
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            (rest, TuiStyle::default(), "• ")
        } else {
            (trimmed, TuiStyle::default(), "")
        };

        let mut spans = Vec::new();
        if !prefix.is_empty() {
            spans.push(Span::styled(prefix.to_string(), style));
        }
        spans.extend(inline_spans(content, style));
        lines.push(Line::from(spans));
    }

    lines
}

/// Split `text` into styled spans, converting `**bold**` and `` `code` ``.
///
/// Unpaired markers are kept as literal text.
fn inline_spans(text: &str, base: TuiStyle) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;

    loop {
        let bold = rest.find("**");
        let code = rest.find('`');

        let (pos, is_bold) = match (bold, code) {
            (Some(b), Some(c)) if b <= c => (b, true),
            (Some(_), Some(c)) => (c, false),
            (Some(b), None) => (b, true),
            (None, Some(c)) => (c, false),
            (None, None) => break,
        };

        if pos > 0 {
            spans.push(Span::styled(rest[..pos].to_string(), base));
        }

        // Locate the closing marker; unpaired markers stay as literal text.
        let found = if is_bold {
            rest[pos + 2..]
                .find("**")
                .map(|end| (&rest[pos + 2..pos + 2 + end], pos + 2 + end + 2))
        } else {
            rest[pos + 1..]
                .find('`')
                .map(|end| (&rest[pos + 1..pos + 1 + end], pos + 1 + end + 1))
        };

        let Some((inner, next)) = found else {
            // Unpaired marker: keep the remainder as plain text.
            spans.push(Span::styled(rest[pos..].to_string(), base));
            return spans;
        };

        let style = if is_bold {
            base.add_modifier(Modifier::BOLD)
        } else {
            TuiStyle::default().fg(Color::Yellow)
        };
        spans.push(Span::styled(inner.to_string(), style));
        rest = &rest[next..];
    }

    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), base));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_output_preserves_markdown_text() {
        let theme = Theme::default();
        let input = "# Title\n\n**bold** and `code`";
        assert_eq!(markdown_to_ansi(input, &theme, true), input);
    }

    #[test]
    fn renders_common_markdown_blocks() {
        let theme = Theme::default();
        let input =
            "# Title\n- item\n> quote\n\n```rust\nfn main() {}\n```\n\n[link](https://example.com)";
        let output = markdown_to_ansi(input, &theme, false);

        assert!(output.contains("Title"));
        assert!(output.contains("• item"));
        assert!(output.contains("│ quote"));
        assert!(output.contains("┌"));
        assert!(output.contains("fn main() {}"));
        assert!(output.contains("https://example.com"));
    }

    #[test]
    fn renders_inline_links_and_code() {
        let theme = Theme::default();
        let output = markdown_to_ansi(
            "`inline` and [docs](https://docs.example.com)",
            &theme,
            false,
        );

        assert!(output.contains("inline"));
        assert!(output.contains("docs"));
        assert!(output.contains("https://docs.example.com"));
    }

    #[test]
    fn tui_lines_strip_markers_and_keep_styles() {
        let lines = markdown_to_lines("**bold** and `code`");
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "bold and code");
        assert!(lines[0].spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn tui_lines_render_headers_bullets_and_code_blocks() {
        let lines = markdown_to_lines("# Title\n- item\n```sh\nls -la\n```");
        assert_eq!(lines.len(), 3);

        let header: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(header, "Title");
        assert!(lines[0].spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));

        let bullet: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(bullet, "• item");

        let code: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(code, "  ls -la");
    }

    #[test]
    fn tui_lines_keep_unpaired_markers() {
        let lines = markdown_to_lines("2 ** 8 is a power");
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "2 ** 8 is a power");
    }
}
