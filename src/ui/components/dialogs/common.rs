use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

/// Creates a styled main dialog block
pub fn create_dialog_block<'a>(title: &'a str, theme_color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(theme_color).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(theme_color))
}

/// Returns the horizontal scroll offset and on-screen cursor column for an input.
pub fn input_viewport(input_buffer: &str, cursor_position: usize, visible_width: u16) -> (u16, u16) {
    let prefix: String = input_buffer.chars().take(cursor_position).collect();
    let cursor_column = Line::from(prefix).width();
    let last_visible_column = usize::from(visible_width.saturating_sub(1));
    let scroll_offset = cursor_column.saturating_sub(last_visible_column);
    let visible_cursor_column = cursor_column.saturating_sub(scroll_offset);

    (
        u16::try_from(scroll_offset).unwrap_or(u16::MAX),
        u16::try_from(visible_cursor_column).unwrap_or(u16::MAX),
    )
}

/// Hard-wraps multiline input and returns its rendered text plus cursor row/column.
///
/// Keeping wrapping here, rather than delegating it to `Paragraph`, ensures the
/// terminal cursor and displayed text use the same layout rules.
pub fn multiline_input_layout(input_buffer: &str, cursor_position: usize, visible_width: u16) -> (String, u16, u16) {
    let width = usize::from(visible_width.max(1));
    let (rendered, positions) = multiline_layout(input_buffer, width);
    let (row, column) = positions
        .get(cursor_position.min(positions.len().saturating_sub(1)))
        .copied()
        .unwrap_or((0, 0));
    (
        rendered,
        u16::try_from(row).unwrap_or(u16::MAX),
        u16::try_from(column).unwrap_or(u16::MAX),
    )
}

/// Moves a character-index cursor to the closest column on an adjacent
/// visually wrapped row.
pub fn move_multiline_cursor(input_buffer: &str, cursor_position: usize, visible_width: u16, row_delta: i32) -> usize {
    let (_, positions) = multiline_layout(input_buffer, usize::from(visible_width.max(1)));
    let current_index = cursor_position.min(positions.len().saturating_sub(1));
    let (current_row, current_column) = positions[current_index];
    let target_row = if row_delta < 0 {
        current_row.checked_sub(row_delta.unsigned_abs() as usize)
    } else {
        current_row.checked_add(row_delta as usize)
    };
    let Some(target_row) = target_row else {
        return current_index;
    };

    positions
        .iter()
        .enumerate()
        .filter(|(_, (row, _))| *row == target_row)
        .min_by_key(|(_, (_, column))| column.abs_diff(current_column))
        .map(|(index, _)| index)
        .unwrap_or(current_index)
}

fn multiline_layout(input_buffer: &str, width: usize) -> (String, Vec<(usize, usize)>) {
    let mut rendered = String::with_capacity(input_buffer.len());
    let mut positions = Vec::with_capacity(input_buffer.chars().count() + 1);
    let (mut row, mut column) = (0, 0);
    positions.push((row, column));

    for character in input_buffer.chars() {
        if character == '\n' {
            rendered.push('\n');
            row += 1;
            column = 0;
        } else {
            let character_width = Line::from(character.to_string()).width();
            if column > 0 && column + character_width > width {
                rendered.push('\n');
                row += 1;
                column = 0;
            }
            rendered.push(character);
            column += character_width;
            if column >= width {
                rendered.push('\n');
                row += 1;
                column = 0;
            }
        }
        positions.push((row, column));
    }
    (rendered, positions)
}

/// Creates an input field that scrolls horizontally to keep the cursor visible.
pub fn create_input_paragraph<'a>(
    input_buffer: &'a str,
    cursor_position: usize,
    visible_width: u16,
    field_title: &str,
) -> Paragraph<'a> {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", field_title))
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    let (scroll_offset, _) = input_viewport(input_buffer, cursor_position, visible_width);

    Paragraph::new(input_buffer)
        .block(input_block)
        .style(Style::default().fg(Color::White))
        .scroll((0, scroll_offset))
}

/// Creates a selection field block (read-only display with title)
pub fn create_selection_paragraph(value: String, field_title: &str) -> Paragraph<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", field_title))
        .title_style(Style::default().fg(Color::White))
        .style(Style::default().fg(Color::Gray));

    Paragraph::new(value).block(block).style(Style::default().fg(Color::White))
}

/// Instruction shortcut definition: (key, color, description)
pub type InstructionShortcut = (&'static str, Color, &'static str);

/// Creates a paragraph with color-coded instruction shortcuts
pub fn create_instructions_paragraph<'a>(instructions: &[InstructionShortcut]) -> Paragraph<'a> {
    let mut instruction_text = Vec::new();
    for (key, color, desc) in instructions {
        instruction_text.push(Span::styled(
            *key,
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
        instruction_text.push(Span::styled(*desc, Style::default().fg(Color::Gray)));
    }

    Paragraph::new(Line::from(instruction_text)).alignment(Alignment::Center)
}

/// Builds a responsive shortcut footer and reports the rows it requires.
///
/// Shortcut groups stay intact when possible and move to the next line at
/// separators. If one group is wider than the available area, only its key is
/// shown so the footer never relies on horizontal clipping.
pub fn create_responsive_instructions_paragraph(
    instructions: &[InstructionShortcut],
    visible_width: u16,
) -> (Paragraph<'static>, u16) {
    let available_width = usize::from(visible_width.max(1));
    let separator = Span::styled(" • ", Style::default().fg(Color::Gray));
    let separator_width = Line::from(separator.clone()).width();
    let mut groups = Vec::<Vec<Span<'static>>>::new();

    for (key, color, description) in instructions {
        if description.is_empty() {
            continue;
        }
        let full = vec![
            Span::styled(
                (*key).to_string(),
                Style::default().fg(*color).add_modifier(Modifier::BOLD),
            ),
            Span::styled((*description).to_string(), Style::default().fg(Color::Gray)),
        ];
        let group = if Line::from(full.clone()).width() <= available_width {
            full
        } else {
            vec![Span::styled(
                (*key).to_string(),
                Style::default().fg(*color).add_modifier(Modifier::BOLD),
            )]
        };
        groups.push(group);
    }

    let mut lines = Vec::<Line<'static>>::new();
    let mut current = Vec::<Span<'static>>::new();
    let mut current_width = 0;
    for group in groups {
        let group_width = Line::from(group.clone()).width();
        let added_width = if current.is_empty() {
            group_width
        } else {
            separator_width + group_width
        };
        if !current.is_empty() && current_width + added_width > available_width {
            lines.push(Line::from(current).alignment(Alignment::Center));
            current = Vec::new();
            current_width = 0;
        }
        if !current.is_empty() {
            current.push(separator.clone());
            current_width += separator_width;
        }
        current.extend(group);
        current_width += group_width;
    }
    if !current.is_empty() {
        lines.push(Line::from(current).alignment(Alignment::Center));
    }
    if lines.is_empty() {
        lines.push(Line::default());
    }
    let height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    (Paragraph::new(lines), height)
}

/// Common instruction shortcuts used across dialogs
pub mod shortcuts {
    use super::*;

    pub const SEPARATOR: InstructionShortcut = (" • ", Color::Gray, "");
    pub const ESC_CANCEL: InstructionShortcut = ("Esc", Color::Red, " Cancel");
    pub const TAB_SELECT: InstructionShortcut = ("Tab", Color::Cyan, " Select");
}

#[cfg(test)]
mod tests {
    use super::{
        create_responsive_instructions_paragraph, input_viewport, move_multiline_cursor, multiline_input_layout,
    };
    use ratatui::{backend::TestBackend, layout::Rect, style::Color, Terminal};

    #[test]
    fn long_input_scrolls_to_keep_cursor_inside_field() {
        assert_eq!(input_viewport("abcdefghijklmnopqrstuvwxyz", 26, 10), (17, 9));
    }

    #[test]
    fn short_input_does_not_scroll() {
        assert_eq!(input_viewport("task", 4, 10), (0, 4));
    }

    #[test]
    fn viewport_uses_terminal_column_width_for_wide_characters() {
        assert_eq!(input_viewport("ab🙂cd", 5, 5), (2, 4));
    }

    #[test]
    fn multiline_layout_tracks_wrapped_cursor_position() {
        assert_eq!(
            multiline_input_layout("abcdefgh", 8, 5),
            ("abcde\nfgh".to_string(), 1, 3)
        );
    }

    #[test]
    fn multiline_cursor_moves_between_visual_rows() {
        assert_eq!(move_multiline_cursor("abcdefgh", 7, 5, -1), 2);
        assert_eq!(move_multiline_cursor("abcdefgh", 2, 5, 1), 7);
    }

    #[test]
    fn responsive_instructions_add_rows_instead_of_clipping() {
        let instructions = [
            ("j/k", Color::Cyan, " select"),
            ("Space", Color::Cyan, " enable/disable"),
            ("Enter", Color::Green, " continue"),
            ("Esc", Color::Red, " close"),
        ];
        let (paragraph, height) = create_responsive_instructions_paragraph(&instructions, 28);
        assert!(height > 1);

        let backend = TestBackend::new(28, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| frame.render_widget(paragraph, Rect::new(0, 0, 28, height)))
            .unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("j/k"));
        assert!(rendered.contains("Esc"));
    }
}
