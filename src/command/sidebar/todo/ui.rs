//! Rendering and wrapping for the global TODO footer.
use super::TodoPanel;
use crate::ui::theme::ThemePalette;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

impl TodoPanel {
    pub(in crate::command::sidebar) fn height(&self, width: u16) -> u16 {
        if !self.is_visible() {
            return 0;
        }
        if !self.expanded {
            return 1;
        }
        if self.error.is_some() {
            return 2;
        }
        let item_width = width.saturating_sub(2) as usize;
        let visible_rows = self
            .items
            .iter()
            .take(self.max_visible)
            .map(|item| wrap_todo_text(item, item_width, self.max_lines_per_item).len() as u16)
            .fold(0u16, u16::saturating_add);
        let more = u16::from(self.items.len() > self.max_visible);
        1 + visible_rows + more
    }

    pub(in crate::command::sidebar) fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        palette: &ThemePalette,
    ) {
        if area.height == 0 {
            return;
        }

        let marker = if self.expanded { "▾" } else { "▸" };
        let count = self.items.len();
        let header = truncate_to_width(
            &format!(
                "─ {marker} TODO {count} {}",
                "─".repeat(area.width as usize)
            ),
            area.width as usize,
        );
        f.render_widget(
            Line::from(Span::styled(
                header,
                Style::default()
                    .fg(palette.border)
                    .add_modifier(Modifier::BOLD),
            )),
            Rect::new(area.x, area.y, area.width, 1),
        );

        if !self.expanded || area.height == 1 {
            return;
        }

        if let Some(error) = &self.error {
            let text = truncate_to_width(error, area.width.saturating_sub(2) as usize);
            f.render_widget(
                Line::from(vec![
                    Span::styled("! ", Style::default().fg(palette.warning)),
                    Span::styled(text, Style::default().fg(palette.warning)),
                ]),
                Rect::new(area.x, area.y + 1, area.width, 1),
            );
            return;
        }

        let item_width = area.width.saturating_sub(2) as usize;
        let mut row = 1;
        for item in self.items.iter().take(self.max_visible) {
            for (line_index, text) in wrap_todo_text(item, item_width, self.max_lines_per_item)
                .into_iter()
                .enumerate()
            {
                if row >= area.height {
                    break;
                }
                let marker = if line_index == 0 { "□ " } else { "  " };
                f.render_widget(
                    Line::from(vec![
                        Span::styled(marker, Style::default().fg(palette.accent)),
                        Span::styled(text, Style::default().fg(palette.text)),
                    ]),
                    Rect::new(area.x, area.y + row, area.width, 1),
                );
                row += 1;
            }
            if row >= area.height {
                break;
            }
        }

        if self.items.len() > self.max_visible && area.height >= self.height(area.width) {
            let more = format!("+{} more", self.items.len() - self.max_visible);
            f.render_widget(
                Line::from(Span::styled(
                    truncate_to_width(&more, area.width as usize),
                    Style::default().fg(palette.dimmed),
                )),
                Rect::new(area.x, area.bottom() - 1, area.width, 1),
            );
        }
    }
}

fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut width = 0;
    for ch in s.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + ch_width > max_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out
}

fn wrap_todo_text(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    if max_width == 0 || max_lines == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut truncated = false;

    for ch in text.chars().filter(|ch| *ch != '\r') {
        if ch == '\n' {
            lines.push(current.trim_end().to_string());
            current.clear();
            current_width = 0;
            if lines.len() == max_lines {
                truncated = true;
                break;
            }
            continue;
        }

        let char_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + char_width > max_width {
            lines.push(current.trim_end().to_string());
            current.clear();
            current_width = 0;
            if lines.len() == max_lines {
                truncated = true;
                break;
            }
            if ch.is_whitespace() {
                continue;
            }
        }

        current.push(ch);
        current_width += char_width;
    }

    if !truncated && lines.len() < max_lines && (!current.is_empty() || lines.is_empty()) {
        lines.push(current.trim_end().to_string());
    }

    if truncated && let Some(last) = lines.last_mut() {
        *last = ellipsize_to_width(last, max_width);
    }

    lines
}

fn ellipsize_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let mut text = truncate_to_width(text.trim_end(), max_width.saturating_sub(1));
    text.push('…');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn todo_text_supports_manual_newlines_and_unicode_width() {
        assert_eq!(
            wrap_todo_text("第一行\n第二行", 12, 2),
            vec!["第一行".to_string(), "第二行".to_string()]
        );
        assert_eq!(
            wrap_todo_text("一二三四五六七八九", 6, 2),
            vec!["一二三".to_string(), "四五…".to_string()]
        );
        assert_eq!(
            wrap_todo_text("first\nsecond\nthird", 12, 2),
            vec!["first".to_string(), "second…".to_string()]
        );
    }
}
