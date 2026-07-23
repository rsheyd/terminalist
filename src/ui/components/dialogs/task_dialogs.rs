use super::common::{self, shortcuts};
use crate::entities::{project, task};
use crate::icons::IconService;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

pub fn render_task_details_dialog(
    f: &mut Frame,
    area: Rect,
    task: &task::Model,
    project_name: &str,
    scroll_offset: usize,
    scrollbar_state: &mut ScrollbarState,
) {
    let dialog_area = LayoutManager::centered_rect(80, 80, area);
    f.render_widget(Clear, dialog_area);

    let block = common::create_dialog_block("Task Details", Color::Cyan);
    let inner = block.inner(dialog_area);
    let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
        .margin(1)
        .split(inner);
    let content_area = layout[0];

    let status = if task.is_deleted {
        "Deleted"
    } else if task.is_completed {
        "Completed"
    } else {
        "Active"
    };
    let priority = match task.priority {
        4 => "P1 (urgent)",
        3 => "P2 (high)",
        2 => "P3 (medium)",
        _ => "P4 (normal)",
    };
    let due = task.due_datetime.as_deref().or(task.due_date.as_deref()).unwrap_or("None");
    let description = task.description.as_deref().filter(|value| !value.is_empty()).unwrap_or("None");

    let mut lines = vec![
        Line::from(Span::styled(
            task.content.clone(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        detail_line("Status", status),
        detail_line("Project", project_name),
        detail_line("Priority", priority),
        detail_line("Due", due),
        detail_line("Deadline", task.deadline.as_deref().unwrap_or("None")),
        detail_line("Duration", task.duration.as_deref().unwrap_or("None")),
        detail_line("Recurring", if task.is_recurring { "Yes" } else { "No" }),
        Line::from(""),
        Line::from(Span::styled(
            "Description",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
    ];
    lines.extend(description.lines().map(|line| Line::from(line.to_string())));

    let content_length = lines.len();
    let visible_height = content_area.height as usize;
    let max_scroll = content_length.saturating_sub(visible_height);
    let clamped_offset = scroll_offset.min(max_scroll);
    *scrollbar_state = scrollbar_state
        .content_length(content_length)
        .viewport_content_length(visible_height)
        .position(clamped_offset);

    let details = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((clamped_offset as u16, 0));
    let instructions = Paragraph::new("Esc close • j/k or ↑/↓ scroll")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    f.render_widget(block, dialog_area);
    f.render_widget(details, content_area);
    f.render_widget(instructions, layout[1]);

    if content_length > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("▐");
        f.render_stateful_widget(scrollbar, content_area, scrollbar_state);
    }
}

fn detail_line(label: &'static str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_string()),
    ])
}

#[allow(clippy::too_many_arguments)]
pub fn render_task_dialog(
    f: &mut Frame,
    area: Rect,
    _icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    task_projects: &[&project::Model],
    selected_project_index: Option<usize>,
    is_editing: bool,
) {
    let title = if is_editing { "Edit Task" } else { "New Task" };
    let dialog_area = LayoutManager::centered_rect_lines(65, 12, area);
    f.render_widget(Clear, dialog_area);

    let main_block = common::create_dialog_block(title, Color::Cyan);

    // Create layout for content
    let inner_area = main_block.inner(dialog_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // Task content input field (borders + content)
            Constraint::Length(4), // Project selection field (borders + content)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    let input_width = chunks[0].width.saturating_sub(2);
    let input_paragraph = common::create_input_paragraph(input_buffer, cursor_position, input_width, "Task Content");

    // Project selection field
    let project_name = match selected_project_index {
        None => "None (Inbox)".to_string(),
        Some(index) => {
            if index < task_projects.len() {
                task_projects[index].name.clone()
            } else {
                "None (Inbox)".to_string()
            }
        }
    };

    let project_paragraph = common::create_selection_paragraph(project_name, "Project");

    // Instructions based on mode
    let action = if is_editing {
        ("Enter", Color::Green, " Save Task")
    } else {
        ("Enter", Color::Green, " Create Task")
    };

    let instructions = [
        action,
        shortcuts::SEPARATOR,
        shortcuts::TAB_SELECT,
        (" Project", Color::Gray, ""),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ];
    let instructions_paragraph = common::create_instructions_paragraph(&instructions);

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(project_paragraph, chunks[1]);
    f.render_widget(instructions_paragraph, chunks[3]);

    // Set the cursor inside the horizontally scrolled input viewport.
    let (_, visible_cursor_column) = common::input_viewport(input_buffer, cursor_position, input_width);
    f.set_cursor_position((chunks[0].x + 1 + visible_cursor_column, chunks[0].y + 1));
}

// Legacy wrapper functions for backward compatibility
pub fn render_task_creation_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
        cursor_position,
        task_projects,
        selected_task_project_index,
        false, // is_editing = false for creation
    );
}

pub fn render_task_edit_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
        cursor_position,
        task_projects,
        selected_task_project_index,
        true, // is_editing = true for editing
    );
}

pub fn render_task_time_dialog(f: &mut Frame, area: Rect, input: &str, cursor_position: usize) {
    // Nine rows guarantee one content row inside the bordered input after the
    // dialog's borders and margins are applied.
    let dialog_area = LayoutManager::centered_rect_lines(45, 9, area);
    f.render_widget(Clear, dialog_area);
    let block = common::create_dialog_block("Set Due Time", Color::Cyan);
    let inner = block.inner(dialog_area);
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(inner);
    let input_width = chunks[0].width.saturating_sub(2);
    let field = common::create_input_paragraph(input, cursor_position, input_width, "Time (e.g. 2pm)");
    let instructions = common::create_instructions_paragraph(&[
        ("Enter", Color::Green, " Save"),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ]);
    f.render_widget(block, dialog_area);
    f.render_widget(field, chunks[0]);
    f.render_widget(instructions, chunks[2]);
    let (_, cursor) = common::input_viewport(input, cursor_position, input_width);
    f.set_cursor_position((chunks[0].x + 1 + cursor, chunks[0].y + 1));
}
