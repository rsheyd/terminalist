use super::common::{self, shortcuts};
use crate::entities::{project, task};
use crate::icons::IconService;
use crate::ui::core::{AiAssistStage, AiExecutionReport, AiProposalPage, AiTaskProposal};
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
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
    let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(2)])
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
    let priority = crate::priority::TaskPriority::from_todoist(task.priority).label();
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
    let instructions = Paragraph::new("e edit title • m manage with AI • Esc close\nj/k or ↑/↓ scroll")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

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

#[allow(clippy::too_many_arguments)]
pub fn render_ai_task_management_dialog(
    f: &mut Frame,
    area: Rect,
    task: &task::Model,
    stage: AiAssistStage,
    input_buffer: &str,
    cursor_position: usize,
    proposal: Option<&AiTaskProposal>,
    proposal_page: AiProposalPage,
    recommendation_scroll: usize,
    selected_action_index: usize,
    error_message: Option<&str>,
    report: Option<&AiExecutionReport>,
) -> usize {
    let dialog_area = LayoutManager::centered_rect(82, 78, area);
    f.render_widget(Clear, dialog_area);

    let block = common::create_dialog_block("AI Task Management", Color::Magenta);
    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let mut recommendation_max_scroll = 0;
    match stage {
        AiAssistStage::ContextEntry => {
            let layout = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(5),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .margin(1)
            .split(inner);

            let task_title = Paragraph::new(vec![
                Line::from(Span::styled(
                    task.content.clone(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from("What has changed, and what would you like help deciding?"),
            ]);
            let input_width = layout[1].width.saturating_sub(2);
            let input_height = layout[1].height.saturating_sub(2);
            let (wrapped_input, cursor_row, cursor_column) =
                common::multiline_input_layout(input_buffer, cursor_position, input_width);
            let vertical_scroll = cursor_row.saturating_sub(input_height.saturating_sub(1));
            let context = Paragraph::new(wrapped_input)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Context")
                        .style(Style::default().fg(Color::Gray)),
                )
                .scroll((vertical_scroll, 0));
            let notice = Paragraph::new(error_message.unwrap_or(
                "OpenAI will generate a proposal. Nothing changes in Todoist until you review and confirm it.",
            ))
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: true });
            let instructions = Paragraph::new("Enter generate proposal • Esc cancel")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);

            f.render_widget(task_title, layout[0]);
            f.render_widget(context, layout[1]);
            f.render_widget(notice, layout[2]);
            f.render_widget(instructions, layout[3]);

            f.set_cursor_position((
                layout[1].x + 1 + cursor_column,
                layout[1].y + 1 + cursor_row.saturating_sub(vertical_scroll),
            ));
        }
        AiAssistStage::RevisionEntry => {
            let footer = [("Enter", Color::Green, " regenerate proposal"), ("Esc", Color::Red, " back")];
            let footer_width = inner.width.saturating_sub(2);
            let (instructions, footer_height) = common::create_responsive_instructions_paragraph(&footer, footer_width);
            let layout = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(5),
                Constraint::Length(2),
                Constraint::Length(footer_height),
            ])
            .margin(1)
            .split(inner);
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        task.content.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    )),
                    Line::from("What should change in the proposed actions?"),
                ]),
                layout[0],
            );
            let input_width = layout[1].width.saturating_sub(2);
            let input_height = layout[1].height.saturating_sub(2);
            let (wrapped_input, cursor_row, cursor_column) =
                common::multiline_input_layout(input_buffer, cursor_position, input_width);
            let vertical_scroll = cursor_row.saturating_sub(input_height.saturating_sub(1));
            f.render_widget(
                Paragraph::new(wrapped_input)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Revision request")
                            .style(Style::default().fg(Color::Gray)),
                    )
                    .scroll((vertical_scroll, 0)),
                layout[1],
            );
            f.render_widget(
                Paragraph::new("OpenAI will replace the current proposal; no Todoist changes occur.")
                    .style(Style::default().fg(Color::Yellow))
                    .wrap(Wrap { trim: true }),
                layout[2],
            );
            f.render_widget(instructions, layout[3]);
            f.set_cursor_position((
                layout[1].x + 1 + cursor_column,
                layout[1].y + 1 + cursor_row.saturating_sub(vertical_scroll),
            ));
        }
        AiAssistStage::Generating => {
            let layout = Layout::vertical([Constraint::Min(3)]).margin(2).split(inner);
            f.render_widget(
                Paragraph::new(
                    "Generating a task-management proposal with OpenAI…\n\nNo Todoist changes are being made.",
                )
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
                layout[0],
            );
        }
        AiAssistStage::ProposalReview => {
            let footer_instructions = match proposal_page {
                AiProposalPage::Recommendation => vec![
                    ("←/→", Color::Cyan, " change page"),
                    ("j/k or ↑/↓", Color::Cyan, " scroll"),
                    ("Enter", Color::Green, " actions"),
                    ("Esc", Color::Red, " close"),
                ],
                AiProposalPage::Actions => vec![
                    ("←/→", Color::Cyan, " change page"),
                    ("j/k or ↑/↓", Color::Cyan, " select"),
                    ("Space", Color::Cyan, " enable/disable"),
                    ("r", Color::Magenta, " revise"),
                    ("Enter", Color::Green, " continue"),
                    ("Esc", Color::Red, " close"),
                ],
            };
            let footer_width = inner.width.saturating_sub(2);
            let (instructions, footer_height) =
                common::create_responsive_instructions_paragraph(&footer_instructions, footer_width);
            match proposal_page {
                AiProposalPage::Recommendation => {
                    let layout = Layout::vertical([
                        Constraint::Length(2),
                        Constraint::Min(6),
                        Constraint::Length(footer_height),
                    ])
                    .margin(1)
                    .split(inner);
                    render_proposal_heading(f, layout[0], task, "Recommendation · 1/2");
                    if let Some(proposal) = proposal {
                        let content_width = layout[1].width.saturating_sub(2);
                        let content_height = layout[1].height.saturating_sub(2);
                        let (wrapped, last_row, _) = common::multiline_input_layout(
                            &proposal.summary,
                            proposal.summary.chars().count(),
                            content_width,
                        );
                        let content_rows = usize::from(last_row) + 1;
                        recommendation_max_scroll = content_rows.saturating_sub(usize::from(content_height));
                        let visible_scroll = recommendation_scroll.min(recommendation_max_scroll);
                        let summary = Paragraph::new(wrapped)
                            .block(Block::default().borders(Borders::ALL).title("Recommendation"))
                            .scroll((u16::try_from(visible_scroll).unwrap_or(u16::MAX), 0));
                        f.render_widget(summary, layout[1]);
                    }
                    f.render_widget(instructions, layout[2]);
                }
                AiProposalPage::Actions => {
                    let layout = Layout::vertical([
                        Constraint::Length(2),
                        Constraint::Min(6),
                        Constraint::Length(3),
                        Constraint::Length(footer_height),
                    ])
                    .margin(1)
                    .split(inner);
                    render_proposal_heading(f, layout[0], task, "Proposed Actions · 2/2");
                    if let Some(proposal) = proposal {
                        let action_count = proposal.actions.len();
                        let item_width = layout[1].width.saturating_sub(2);
                        let selected_index = selected_action_index.min(action_count.saturating_sub(1));
                        let mut action_lines = Vec::new();
                        let mut selected_start = 0usize;
                        let mut selected_height = 0usize;
                        for (index, action) in proposal.actions.iter().enumerate() {
                            if index == selected_index {
                                selected_start = action_lines.len();
                            }
                            let marker = if action.enabled { "[x]" } else { "[ ]" };
                            let description = format!("{marker} {}", action.description());
                            let (wrapped, _, _) =
                                common::multiline_input_layout(&description, description.chars().count(), item_width);
                            let style = if index == selected_action_index {
                                Style::default().fg(Color::Black).bg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            let wrapped_lines = wrapped
                                .split('\n')
                                .map(|line| Line::styled(line.to_string(), style))
                                .collect::<Vec<_>>();
                            let wrapped_height = wrapped_lines.len();
                            action_lines.extend(wrapped_lines);
                            if index == selected_index {
                                selected_height = wrapped_height;
                            }
                        }
                        let viewport_height = usize::from(layout[1].height.saturating_sub(2));
                        let total_rows = action_lines.len();
                        let max_scroll = total_rows.saturating_sub(viewport_height);
                        let selected_end = selected_start.saturating_add(selected_height);
                        let row_scroll = if selected_height >= viewport_height {
                            selected_start
                        } else {
                            selected_end.saturating_sub(viewport_height)
                        }
                        .min(max_scroll);
                        let action_list = Paragraph::new(action_lines)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .title(format!("Proposed actions ({action_count})")),
                            )
                            .scroll((u16::try_from(row_scroll).unwrap_or(u16::MAX), 0));
                        f.render_widget(action_list, layout[1]);
                        if total_rows > viewport_height {
                            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                                .begin_symbol(Some("↑"))
                                .end_symbol(Some("↓"))
                                .track_symbol(Some("│"))
                                .thumb_symbol("▐");
                            let mut scrollbar_state = ScrollbarState::new(total_rows)
                                .viewport_content_length(viewport_height)
                                .position(row_scroll);
                            f.render_stateful_widget(scrollbar, layout[1], &mut scrollbar_state);
                        }
                    } else {
                        f.render_widget(Paragraph::new("No proposal available."), layout[1]);
                    }
                    let progress_notice = proposal.map(|proposal| {
                        let count = proposal.actions.len();
                        if count == 0 {
                            "No proposed actions.".to_string()
                        } else if selected_action_index + 1 < count {
                            format!("Action {} of {count} · ↓ More actions below", selected_action_index + 1)
                        } else {
                            format!("Action {count} of {count} · End of actions")
                        }
                    });
                    let notice_text = error_message
                        .map(str::to_string)
                        .or(progress_notice)
                        .unwrap_or_else(|| "Review each action before continuing.".to_string());
                    let notice_color = if error_message.is_some() {
                        Color::Red
                    } else {
                        Color::Yellow
                    };
                    f.render_widget(
                        Paragraph::new(notice_text)
                            .style(Style::default().fg(notice_color))
                            .wrap(Wrap { trim: true }),
                        layout[2],
                    );
                    f.render_widget(instructions, layout[3]);
                }
            }
        }
        AiAssistStage::ApplyConfirmation => {
            let enabled_count = proposal
                .map(|proposal| proposal.actions.iter().filter(|action| action.enabled).count())
                .unwrap_or(0);
            let layout = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(5),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .margin(1)
            .split(inner);
            f.render_widget(
                Paragraph::new(Span::styled(
                    task.content.clone(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                layout[0],
            );
            f.render_widget(
                Paragraph::new(format!(
                    "Apply {enabled_count} approved action(s) to Todoist?\n\nActions run in safety order: preserve context, create successors, then complete the original task. Execution stops on the first failure."
                ))
                .block(Block::default().borders(Borders::ALL).title("Confirm changes"))
                .wrap(Wrap { trim: false }),
                layout[1],
            );
            f.render_widget(
                Paragraph::new("Todoist cannot apply the whole proposal atomically.")
                    .style(Style::default().fg(Color::Yellow)),
                layout[2],
            );
            f.render_widget(
                Paragraph::new("Enter apply proposal • Esc back")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(Alignment::Center),
                layout[3],
            );
        }
        AiAssistStage::Applying => {
            let layout = Layout::vertical([Constraint::Min(3)]).margin(2).split(inner);
            f.render_widget(
                Paragraph::new("Applying approved actions to Todoist…\n\nThe original task will be completed last.")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                layout[0],
            );
        }
        AiAssistStage::Result => {
            let layout = Layout::vertical([Constraint::Min(5), Constraint::Length(1)])
                .margin(1)
                .split(inner);
            let lines = match report {
                Some(report) => {
                    let mut lines = vec![Line::from(Span::styled(
                        if report.failure.is_some() {
                            "Proposal stopped before all actions completed."
                        } else {
                            "Proposal applied successfully."
                        },
                        Style::default()
                            .fg(if report.failure.is_some() {
                                Color::Yellow
                            } else {
                                Color::Green
                            })
                            .add_modifier(Modifier::BOLD),
                    ))];
                    lines.push(Line::from(""));
                    lines.push(Line::from("Completed actions:"));
                    if report.completed_actions.is_empty() {
                        lines.push(Line::from("• None"));
                    } else {
                        lines.extend(report.completed_actions.iter().map(|action| Line::from(format!("• {action}"))));
                    }
                    if let Some(failure) = &report.failure {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            format!("Stopped: {failure}"),
                            Style::default().fg(Color::Red),
                        )));
                    }
                    lines
                }
                None => vec![Line::from("No execution report available.")],
            };
            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Result"))
                    .wrap(Wrap { trim: false }),
                layout[0],
            );
            f.render_widget(
                Paragraph::new("Enter or Esc close")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(Alignment::Center),
                layout[1],
            );
        }
    }
    recommendation_max_scroll
}

fn render_proposal_heading(f: &mut Frame, area: Rect, task: &task::Model, page_title: &str) {
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                task.content.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                page_title.to_string(),
                Style::default().fg(Color::Magenta),
            )),
        ]),
        area,
    );
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
