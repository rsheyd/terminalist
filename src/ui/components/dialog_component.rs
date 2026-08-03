//! Modal dialog component for various user interactions.
//!
//! This component provides a flexible modal dialog system that handles different
//! types of user interactions including task creation/editing, project management,
//! label management, and system functions like search and debugging.

use crate::config::DisplayConfig;
use crate::entities::{label, project, task};
use crate::icons::IconService;
use crate::sync::SyncService;
use crate::ui::components::task_list_item_component::{ListItem as TaskListItem, TaskItem};
use crate::ui::core::{
    actions::{Action, AiAssistStage, AiProposalPage, DialogType},
    Component,
};
use crate::ui::layout::LayoutManager;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, widgets::ScrollbarState, Frame};
use uuid::Uuid;

use crate::ui::components::dialogs::{
    common, label_dialogs, project_dialogs, scroll_behavior, system_dialogs, task_dialogs,
};

/// Modal dialog component that handles various user interactions.
///
/// This component serves as a container for different types of dialogs:
///
/// # Dialog Types
/// - **Task dialogs** - Create, edit, and manage tasks
/// - **Project dialogs** - Create and manage projects
/// - **Label dialogs** - Create and manage labels
/// - **System dialogs** - Search, logs, help, and confirmation dialogs
///
/// # Features
/// - Input handling with cursor management
/// - Scrolling support for long content
/// - Project/label selection interfaces
/// - Search functionality with live results
/// - Integration with sync service for immediate updates
/// - Configurable display options
///
/// The component delegates specific dialog rendering and logic to specialized
/// dialog modules while providing common infrastructure like input handling
/// and state management.
pub struct DialogComponent {
    pub dialog_type: Option<DialogType>,
    pub input_buffer: String,
    pub task_schedule_buffer: String,
    pub task_schedule_focused: bool,
    pub cursor_position: usize,
    pub projects: Vec<project::Model>,
    pub labels: Vec<label::Model>,
    pub tasks: Vec<task::Model>,
    pub selected_project_index: usize,
    pub selected_parent_project_index: Option<usize>, // For project creation parent selection
    pub selected_task_project_index: Option<usize>,   // For task creation project selection (None = no project/inbox)
    pub selected_task_project_uuid: Option<Uuid>,     // Store the actual UUID to avoid index issues
    pub task_project_explicitly_selected: bool,       // Track if user explicitly selected a project via Tab
    pub icons: IconService,
    // Scrolling support for long content dialogs
    pub scroll_offset: usize,
    pub scrollbar_state: ScrollbarState,
    // Task search state
    pub search_results: Vec<task::Model>,
    pub search_selected_index: usize,
    pub search_results_focused: bool,
    pub ai_selected_action_index: usize,
    pub ai_proposal_page: AiProposalPage,
    pub ai_recommendation_scroll: usize,
    ai_recommendation_max_scroll: usize,
    ai_original_context: String,
    pub ai_error_message: Option<String>,
    ai_input_width: u16,
    pub sync_service: Option<SyncService>,
    pub display_config: DisplayConfig,
}

impl Default for DialogComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogComponent {
    pub fn new() -> Self {
        Self {
            dialog_type: None,
            input_buffer: String::new(),
            task_schedule_buffer: String::new(),
            task_schedule_focused: false,
            cursor_position: 0,
            projects: Vec::new(),
            labels: Vec::new(),
            tasks: Vec::new(),
            selected_project_index: 0,
            selected_parent_project_index: None,
            selected_task_project_index: None, // Default to "None" for tasks (no project)
            selected_task_project_uuid: None,  // No project selected initially
            task_project_explicitly_selected: false, // User hasn't used Tab yet
            icons: IconService::default(),
            scroll_offset: 0,
            scrollbar_state: ScrollbarState::new(0),
            search_results: Vec::new(),
            search_selected_index: 0,
            search_results_focused: false,
            ai_selected_action_index: 0,
            ai_proposal_page: AiProposalPage::Recommendation,
            ai_recommendation_scroll: 0,
            ai_recommendation_max_scroll: 0,
            ai_original_context: String::new(),
            ai_error_message: None,
            ai_input_width: 74,
            sync_service: None,
            display_config: DisplayConfig::default(),
        }
    }

    pub fn update_display_config(&mut self, display_config: DisplayConfig) {
        self.display_config = display_config;
    }

    pub fn update_data(&mut self, projects: Vec<project::Model>, labels: Vec<label::Model>) {
        self.projects = projects;
        self.labels = labels;
    }

    pub fn update_data_with_tasks(
        &mut self,
        projects: Vec<project::Model>,
        labels: Vec<label::Model>,
        tasks: Vec<task::Model>,
    ) {
        self.projects = projects;
        self.labels = labels;
        self.tasks = tasks;
    }

    pub fn set_sync_service(&mut self, sync_service: SyncService) {
        self.sync_service = Some(sync_service);
    }

    /// Get root projects (projects without a parent) for parent selection
    pub fn get_root_projects(&self) -> Vec<&project::Model> {
        self.projects.iter().filter(|project| project.parent_uuid.is_none()).collect()
    }

    /// Get all non-inbox projects for task creation (excludes inbox project)
    pub fn get_task_projects(&self) -> Vec<&project::Model> {
        self.projects.iter().filter(|project| !project.is_inbox_project).collect()
    }

    /// Trigger a database search based on current input
    fn trigger_search(&mut self) -> Action {
        // Trigger background database search
        Action::SearchTasks(self.input_buffer.clone())
    }

    /// Update search results from database query results
    pub fn update_search_results(&mut self, query: &str, results: Vec<task::Model>) {
        // Only update if this is for the current query (avoid race conditions)
        if query == self.input_buffer {
            self.search_results = results;
            self.search_selected_index = self.search_selected_index.min(self.search_results.len().saturating_sub(1));
            if self.search_results.is_empty() {
                self.search_results_focused = false;
            }
        }
    }

    pub fn is_visible(&self) -> bool {
        self.dialog_type.is_some()
    }

    fn handle_submit(&mut self) -> Action {
        match &self.dialog_type {
            Some(DialogType::TaskCreation {
                default_project_uuid,
                default_due_date,
                default_label_uuid,
            }) => {
                if !self.input_buffer.is_empty() {
                    // Determine the project UUID based on whether user explicitly selected via Tab
                    let project_uuid = if self.task_project_explicitly_selected {
                        // User pressed Tab - use their selection (could be None for Inbox or Some(uuid) for a project)
                        self.selected_task_project_uuid
                    } else {
                        // User didn't press Tab - use default project
                        *default_project_uuid
                    };

                    // Debug logging
                    if let Some(ref pid) = project_uuid {
                        let proj_name = self
                            .projects
                            .iter()
                            .find(|p| &p.uuid == pid)
                            .map(|p| p.name.as_str())
                            .unwrap_or("unknown");
                        log::info!("Creating task in project: {} ({})", proj_name, pid);
                    } else {
                        log::info!("Creating task in inbox (no project)");
                    }

                    let action = Action::CreateTask {
                        content: self.input_buffer.clone(),
                        project_uuid,
                        due_string: (!self.task_schedule_buffer.trim().is_empty())
                            .then(|| self.task_schedule_buffer.trim().to_string()),
                        due_date: self
                            .task_schedule_buffer
                            .trim()
                            .is_empty()
                            .then(|| default_due_date.clone())
                            .flatten(),
                        label_uuid: *default_label_uuid,
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::TaskEdit { task_uuid, .. }) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::EditTask {
                        task_uuid: *task_uuid,
                        content: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::TaskDescriptionEdit { task_uuid, .. }) => {
                let action = Action::EditTaskDescription {
                    task_uuid: *task_uuid,
                    description: self.input_buffer.clone(),
                };
                self.clear_dialog();
                action
            }
            Some(DialogType::TaskTime { task_uuid, .. }) => {
                match crate::utils::datetime::today_at_time(&self.input_buffer) {
                    Ok(due_datetime) => {
                        let action = Action::SetTaskDueTime {
                            task_uuid: *task_uuid,
                            due_datetime,
                        };
                        self.clear_dialog();
                        action
                    }
                    Err(message) => Action::ShowDialog(DialogType::Error(message)),
                }
            }
            Some(DialogType::AiTaskManagement {
                task,
                stage: AiAssistStage::ContextEntry,
                ..
            }) => {
                if self.input_buffer.trim().is_empty() {
                    Action::None
                } else {
                    Action::AiAssist {
                        task: task.clone(),
                        context: self.input_buffer.clone(),
                    }
                }
            }
            Some(DialogType::AiTaskManagement {
                task,
                stage: AiAssistStage::RevisionEntry,
                proposal: Some(proposal),
                ..
            }) => {
                if self.input_buffer.trim().is_empty() {
                    Action::None
                } else {
                    Action::AiReviseProposal {
                        task: task.clone(),
                        original_context: self.ai_original_context.clone(),
                        proposal: proposal.clone(),
                        revision: self.input_buffer.clone(),
                    }
                }
            }
            Some(DialogType::ProjectCreation) => {
                if !self.input_buffer.is_empty() {
                    let parent_uuid = if let Some(parent_index) = self.selected_parent_project_index {
                        let root_projects = self.get_root_projects();
                        if parent_index < root_projects.len() {
                            Some(root_projects[parent_index].uuid)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let action = Action::CreateProject {
                        name: self.input_buffer.clone(),
                        parent_uuid,
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::ProjectEdit { project_uuid, .. }) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::EditProject {
                        project_uuid: *project_uuid,
                        name: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::LabelCreation) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::CreateLabel {
                        name: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::LabelEdit { label_uuid, .. }) => {
                if !self.input_buffer.is_empty() {
                    let action = Action::EditLabel {
                        label_uuid: *label_uuid,
                        name: self.input_buffer.clone(),
                    };
                    self.clear_dialog();
                    action
                } else {
                    Action::None
                }
            }
            Some(DialogType::DeleteConfirmation { item_type, item_uuid }) => match item_type.as_str() {
                "task" => {
                    let action = Action::DeleteTask(*item_uuid);
                    self.clear_dialog();
                    action
                }
                "project" => {
                    let action = Action::DeleteProject(*item_uuid);
                    self.clear_dialog();
                    action
                }
                "label" => {
                    let action = Action::DeleteLabel(*item_uuid);
                    self.clear_dialog();
                    action
                }
                _ => Action::None,
            },
            Some(DialogType::EmptyTrashConfirmation { .. }) => {
                self.clear_dialog();
                Action::EmptyTrash
            }
            _ => Action::None,
        }
    }

    fn clear_dialog(&mut self) {
        self.dialog_type = None;
        self.input_buffer.clear();
        self.task_schedule_buffer.clear();
        self.task_schedule_focused = false;
        self.cursor_position = 0;
        self.selected_project_index = 0;
        self.selected_parent_project_index = None;
        self.selected_task_project_index = None; // Reset to "None" for task creation
        self.selected_task_project_uuid = None; // Reset stored UUID
        self.task_project_explicitly_selected = false; // Reset selection flag
        self.scroll_offset = 0;
        self.scrollbar_state = ScrollbarState::new(0);
        self.search_results.clear();
        self.ai_selected_action_index = 0;
        self.ai_error_message = None;
    }

    fn scroll_up(&mut self) {
        scroll_behavior::scroll_up(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn scroll_down(&mut self) {
        scroll_behavior::scroll_down(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn page_up(&mut self) {
        scroll_behavior::page_up(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn page_down(&mut self) {
        scroll_behavior::page_down(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn scroll_to_top(&mut self) {
        scroll_behavior::scroll_to_top(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn scroll_to_bottom(&mut self) {
        scroll_behavior::scroll_to_bottom(&mut self.scroll_offset, &mut self.scrollbar_state);
    }

    fn render_task_creation_dialog(&self, f: &mut Frame, area: Rect) {
        let task_projects = self.get_task_projects();
        task_dialogs::render_task_creation_dialog(
            f,
            area,
            &self.icons,
            &self.input_buffer,
            &self.task_schedule_buffer,
            self.task_schedule_focused,
            self.cursor_position,
            &task_projects,
            self.selected_task_project_index,
        );
    }

    fn render_project_creation_dialog(&self, f: &mut Frame, area: Rect) {
        let root_projects = self.get_root_projects();
        project_dialogs::render_project_creation_dialog(
            f,
            area,
            &self.icons,
            &self.input_buffer,
            self.cursor_position,
            &root_projects,
            self.selected_parent_project_index,
        );
    }

    fn render_project_edit_dialog(&self, f: &mut Frame, area: Rect) {
        project_dialogs::render_project_edit_dialog(f, area, &self.icons, &self.input_buffer, self.cursor_position);
    }

    fn render_label_creation_dialog(&self, f: &mut Frame, area: Rect) {
        label_dialogs::render_label_creation_dialog(f, area, &self.icons, &self.input_buffer, self.cursor_position);
    }

    fn render_label_edit_dialog(&self, f: &mut Frame, area: Rect) {
        label_dialogs::render_label_edit_dialog(f, area, &self.icons, &self.input_buffer, self.cursor_position);
    }

    fn render_task_edit_dialog(&self, f: &mut Frame, area: Rect) {
        let task_projects = self.get_task_projects();

        // Find the current project index for the task being edited
        let current_project_index = if let Some(DialogType::TaskEdit { project_uuid, .. }) = &self.dialog_type {
            task_projects.iter().position(|p| p.uuid == *project_uuid)
        } else {
            None
        };

        task_dialogs::render_task_edit_dialog(
            f,
            area,
            &self.icons,
            &self.input_buffer,
            self.cursor_position,
            &task_projects,
            current_project_index,
        );
    }

    fn render_ai_task_management_dialog(
        &mut self,
        f: &mut Frame,
        area: Rect,
        task: &task::Model,
        stage: AiAssistStage,
        proposal: Option<&crate::ui::core::AiTaskProposal>,
        report: Option<&crate::ui::core::AiExecutionReport>,
    ) {
        self.ai_input_width = LayoutManager::centered_rect(82, 78, area).width.saturating_sub(6);
        self.ai_recommendation_max_scroll = task_dialogs::render_ai_task_management_dialog(
            f,
            area,
            task,
            stage,
            &self.input_buffer,
            self.cursor_position,
            proposal,
            self.ai_proposal_page,
            self.ai_recommendation_scroll,
            self.ai_selected_action_index,
            self.ai_error_message.as_deref(),
            report,
        );
    }

    fn render_delete_confirmation_dialog(&self, f: &mut Frame, area: Rect, item_type: &str) {
        system_dialogs::render_delete_confirmation_dialog(f, area, &self.icons, item_type);
    }

    fn render_info_dialog(&mut self, f: &mut Frame, area: Rect, message: &str) {
        system_dialogs::render_info_dialog(
            f,
            area,
            &self.icons,
            message,
            self.scroll_offset,
            &mut self.scrollbar_state,
        );
    }

    fn render_error_dialog(&mut self, f: &mut Frame, area: Rect, message: &str) {
        system_dialogs::render_error_dialog(
            f,
            area,
            &self.icons,
            message,
            self.scroll_offset,
            &mut self.scrollbar_state,
        );
    }

    fn render_help_dialog(&mut self, f: &mut Frame, area: Rect) {
        system_dialogs::render_help_dialog(f, area, self.scroll_offset, &mut self.scrollbar_state);
    }

    fn render_task_search_dialog(&self, f: &mut Frame, area: Rect) {
        use ratatui::{
            layout::{Constraint, Layout, Margin},
            style::{Color, Style},
            widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
        };

        // Create a centered popup area
        let popup_area = {
            let popup_layout =
                Layout::vertical([Constraint::Percentage(10), Constraint::Min(20), Constraint::Percentage(10)])
                    .split(area);

            Layout::horizontal([Constraint::Percentage(10), Constraint::Min(60), Constraint::Percentage(10)])
                .split(popup_layout[1])[1]
        };

        // Clear the area
        f.render_widget(Clear, popup_area);

        // Split into input, results, and search-specific shortcut areas
        let content_area = popup_area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        let layout = Layout::vertical([
            Constraint::Length(3), // Input area
            Constraint::Min(0),    // Results area
            Constraint::Length(1), // Search shortcuts
        ])
        .split(content_area);

        // Render the main block
        let main_block = Block::default()
            .title(" Search Tasks ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(main_block, popup_area);

        // Render input field
        let input_paragraph = Paragraph::new(self.input_buffer.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Query")
                .style(Style::default().fg(Color::Gray)),
        );
        f.render_widget(input_paragraph, layout[0]);

        // Show the terminal cursor only while the query field has focus.
        if !self.search_results_focused {
            f.set_cursor_position((layout[0].x + 1 + self.cursor_position as u16, layout[0].y + 1));
        }

        // Render search results
        let results_text = if self.search_results.is_empty() {
            if self.input_buffer.is_empty() {
                "Start typing to search tasks…".to_string()
            } else {
                "No tasks found.".to_string()
            }
        } else {
            format!("{} tasks found", self.search_results.len())
        };

        let results_list: Vec<ListItem> = self
            .search_results
            .iter()
            .enumerate()
            .map(|(index, task)| {
                // TODO: Load task-label relationships from database
                let task_labels = Vec::new();

                // Create TaskItem with the same formatting as main task list
                let task_item = TaskItem::new(
                    task.clone(),
                    0, // depth: 0 for search results (no indentation)
                    0, // child_count: 0 for search results
                    self.icons.clone(),
                    self.projects.clone(),
                    task_labels,
                );

                // Use the same render method as main task list
                TaskListItem::render(
                    &task_item,
                    self.search_results_focused && self.search_selected_index == index,
                    &self.display_config,
                )
            })
            .collect();

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(results_text)
            .style(Style::default().fg(Color::Gray));

        let results_list_widget = List::new(results_list).block(results_block);
        f.render_widget(results_list_widget, layout[1]);

        let instructions = if self.search_results_focused {
            vec![
                ("j/k or ↑/↓", Color::Cyan, " Navigate"),
                common::shortcuts::SEPARATOR,
                ("Space", Color::Cyan, " Toggle complete"),
                common::shortcuts::SEPARATOR,
                ("t", Color::Cyan, " Today"),
                common::shortcuts::SEPARATOR,
                ("Esc", Color::Red, " Close"),
            ]
        } else {
            vec![
                ("Type", Color::Cyan, " Search"),
                common::shortcuts::SEPARATOR,
                ("↓", Color::Cyan, " Results"),
                common::shortcuts::SEPARATOR,
                ("Esc", Color::Red, " Close"),
            ]
        };
        f.render_widget(common::create_instructions_paragraph(&instructions), layout[2]);
    }

    fn render_logs_dialog(&mut self, f: &mut Frame, area: Rect) {
        system_dialogs::render_logs_dialog(f, area, self.scroll_offset, &mut self.scrollbar_state);
    }

    fn handle_ai_task_management_key(&mut self, key: KeyEvent) -> Action {
        let stage = match &self.dialog_type {
            Some(DialogType::AiTaskManagement { stage, .. }) => *stage,
            _ => return Action::None,
        };

        match stage {
            AiAssistStage::ContextEntry => match key.code {
                KeyCode::Esc => Action::HideDialog,
                KeyCode::Enter => {
                    self.ai_original_context = self.input_buffer.clone();
                    self.handle_submit()
                }
                KeyCode::Char(c) => {
                    let byte_pos: usize = self
                        .input_buffer
                        .chars()
                        .take(self.cursor_position)
                        .map(|ch| ch.len_utf8())
                        .sum();
                    self.input_buffer.insert(byte_pos, c);
                    self.cursor_position += 1;
                    Action::None
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        let byte_pos: usize = self
                            .input_buffer
                            .chars()
                            .take(self.cursor_position)
                            .map(|ch| ch.len_utf8())
                            .sum();
                        let prev_char_len = self
                            .input_buffer
                            .chars()
                            .nth(self.cursor_position - 1)
                            .map(|ch| ch.len_utf8())
                            .unwrap_or(1);
                        self.input_buffer.remove(byte_pos - prev_char_len);
                        self.cursor_position -= 1;
                    }
                    Action::None
                }
                KeyCode::Delete => {
                    if self.cursor_position < self.input_buffer.chars().count() {
                        let byte_pos: usize = self
                            .input_buffer
                            .chars()
                            .take(self.cursor_position)
                            .map(|ch| ch.len_utf8())
                            .sum();
                        self.input_buffer.remove(byte_pos);
                    }
                    Action::None
                }
                KeyCode::Left => {
                    self.cursor_position = self.cursor_position.saturating_sub(1);
                    Action::None
                }
                KeyCode::Right => {
                    if self.cursor_position < self.input_buffer.chars().count() {
                        self.cursor_position += 1;
                    }
                    Action::None
                }
                KeyCode::Up | KeyCode::Down => {
                    let row_delta = if key.code == KeyCode::Up { -1 } else { 1 };
                    self.cursor_position = common::move_multiline_cursor(
                        &self.input_buffer,
                        self.cursor_position,
                        self.ai_input_width,
                        row_delta,
                    );
                    Action::None
                }
                _ => Action::None,
            },
            AiAssistStage::RevisionEntry => match key.code {
                KeyCode::Esc => {
                    if let Some(DialogType::AiTaskManagement { stage, .. }) = self.dialog_type.as_mut() {
                        *stage = AiAssistStage::ProposalReview;
                    }
                    self.ai_proposal_page = AiProposalPage::Actions;
                    self.input_buffer.clear();
                    self.cursor_position = 0;
                    Action::None
                }
                KeyCode::Enter => self.handle_submit(),
                KeyCode::Char(c) => {
                    let byte_pos = self
                        .input_buffer
                        .chars()
                        .take(self.cursor_position)
                        .map(|ch| ch.len_utf8())
                        .sum();
                    self.input_buffer.insert(byte_pos, c);
                    self.cursor_position += 1;
                    Action::None
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        let byte_pos: usize = self
                            .input_buffer
                            .chars()
                            .take(self.cursor_position)
                            .map(|ch| ch.len_utf8())
                            .sum();
                        let previous_width = self
                            .input_buffer
                            .chars()
                            .nth(self.cursor_position - 1)
                            .map(|ch| ch.len_utf8())
                            .unwrap_or(1);
                        self.input_buffer.remove(byte_pos - previous_width);
                        self.cursor_position -= 1;
                    }
                    Action::None
                }
                KeyCode::Delete => {
                    if self.cursor_position < self.input_buffer.chars().count() {
                        let byte_pos: usize = self
                            .input_buffer
                            .chars()
                            .take(self.cursor_position)
                            .map(|ch| ch.len_utf8())
                            .sum();
                        self.input_buffer.remove(byte_pos);
                    }
                    Action::None
                }
                KeyCode::Left => {
                    self.cursor_position = self.cursor_position.saturating_sub(1);
                    Action::None
                }
                KeyCode::Right => {
                    if self.cursor_position < self.input_buffer.chars().count() {
                        self.cursor_position += 1;
                    }
                    Action::None
                }
                KeyCode::Up | KeyCode::Down => {
                    let row_delta = if key.code == KeyCode::Up { -1 } else { 1 };
                    self.cursor_position = common::move_multiline_cursor(
                        &self.input_buffer,
                        self.cursor_position,
                        self.ai_input_width,
                        row_delta,
                    );
                    Action::None
                }
                _ => Action::None,
            },
            AiAssistStage::Generating => Action::None,
            AiAssistStage::ProposalReview => {
                let action_count = match &self.dialog_type {
                    Some(DialogType::AiTaskManagement {
                        proposal: Some(proposal),
                        ..
                    }) => proposal.actions.len(),
                    _ => 0,
                };

                match key.code {
                    KeyCode::Esc => Action::HideDialog,
                    KeyCode::Left => {
                        self.ai_proposal_page = AiProposalPage::Recommendation;
                        Action::None
                    }
                    KeyCode::Right => {
                        self.ai_proposal_page = AiProposalPage::Actions;
                        Action::None
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        match self.ai_proposal_page {
                            AiProposalPage::Recommendation => {
                                self.ai_recommendation_scroll = self.ai_recommendation_scroll.saturating_sub(1);
                            }
                            AiProposalPage::Actions => {
                                self.ai_selected_action_index = self.ai_selected_action_index.saturating_sub(1);
                            }
                        }
                        Action::None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match self.ai_proposal_page {
                            AiProposalPage::Recommendation => {
                                self.ai_recommendation_scroll = self
                                    .ai_recommendation_scroll
                                    .saturating_add(1)
                                    .min(self.ai_recommendation_max_scroll);
                            }
                            AiProposalPage::Actions => {
                                if self.ai_selected_action_index + 1 < action_count {
                                    self.ai_selected_action_index += 1;
                                }
                            }
                        }
                        Action::None
                    }
                    KeyCode::Char(' ') => {
                        if self.ai_proposal_page != AiProposalPage::Actions {
                            return Action::None;
                        }
                        if let Some(DialogType::AiTaskManagement {
                            proposal: Some(proposal),
                            ..
                        }) = self.dialog_type.as_mut()
                        {
                            if let Some(action) = proposal.actions.get_mut(self.ai_selected_action_index) {
                                action.enabled = !action.enabled;
                            }
                        }
                        self.ai_error_message = None;
                        Action::None
                    }
                    KeyCode::Char('r') if self.ai_proposal_page == AiProposalPage::Actions => {
                        if let Some(DialogType::AiTaskManagement { stage, .. }) = self.dialog_type.as_mut() {
                            *stage = AiAssistStage::RevisionEntry;
                        }
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.ai_error_message = None;
                        Action::None
                    }
                    KeyCode::Enter => {
                        if self.ai_proposal_page == AiProposalPage::Recommendation {
                            self.ai_proposal_page = AiProposalPage::Actions;
                            return Action::None;
                        }
                        let proposal = match &self.dialog_type {
                            Some(DialogType::AiTaskManagement {
                                proposal: Some(proposal),
                                ..
                            }) => proposal.clone(),
                            _ => return Action::None,
                        };
                        match proposal.validate() {
                            Ok(()) => {
                                if let Some(DialogType::AiTaskManagement { stage, .. }) = self.dialog_type.as_mut() {
                                    *stage = AiAssistStage::ApplyConfirmation;
                                }
                                self.ai_error_message = None;
                            }
                            Err(message) => self.ai_error_message = Some(message),
                        }
                        Action::None
                    }
                    _ => Action::None,
                }
            }
            AiAssistStage::ApplyConfirmation => match key.code {
                KeyCode::Esc => {
                    if let Some(DialogType::AiTaskManagement { stage, .. }) = self.dialog_type.as_mut() {
                        *stage = AiAssistStage::ProposalReview;
                    }
                    Action::None
                }
                KeyCode::Enter => match &self.dialog_type {
                    Some(DialogType::AiTaskManagement {
                        task,
                        proposal: Some(proposal),
                        ..
                    }) => Action::AiApplyProposal {
                        task: task.clone(),
                        proposal: proposal.clone(),
                    },
                    _ => Action::None,
                },
                _ => Action::None,
            },
            AiAssistStage::Applying => Action::None,
            AiAssistStage::Result => match key.code {
                KeyCode::Esc | KeyCode::Enter => Action::HideDialog,
                _ => Action::None,
            },
        }
    }
}

impl Component for DialogComponent {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action {
        if self.dialog_type.is_none() {
            return Action::None;
        }

        if matches!(self.dialog_type, Some(DialogType::AiTaskManagement { .. })) {
            return self.handle_ai_task_management_key(key);
        }

        if matches!(self.dialog_type, Some(DialogType::TaskCreation { .. })) {
            match key.code {
                KeyCode::Esc => return Action::HideDialog,
                KeyCode::Enter => return self.handle_submit(),
                KeyCode::Up | KeyCode::Down => {
                    self.task_schedule_focused = !self.task_schedule_focused;
                    self.cursor_position = if self.task_schedule_focused {
                        self.task_schedule_buffer.chars().count()
                    } else {
                        self.input_buffer.chars().count()
                    };
                    return Action::None;
                }
                KeyCode::Tab => {
                    let projects_data: Vec<(Uuid, String)> = self
                        .get_task_projects()
                        .iter()
                        .map(|project| (project.uuid, project.name.clone()))
                        .collect();
                    if !projects_data.is_empty() {
                        self.task_project_explicitly_selected = true;
                        self.selected_task_project_index = match self.selected_task_project_index {
                            None => {
                                self.selected_task_project_uuid = Some(projects_data[0].0);
                                Some(0)
                            }
                            Some(index) => {
                                let next_index = (index + 1) % (projects_data.len() + 1);
                                if next_index == projects_data.len() {
                                    self.selected_task_project_uuid = None;
                                    None
                                } else {
                                    self.selected_task_project_uuid = Some(projects_data[next_index].0);
                                    Some(next_index)
                                }
                            }
                        };
                    }
                    return Action::None;
                }
                KeyCode::Char(character) => {
                    let buffer = if self.task_schedule_focused {
                        &mut self.task_schedule_buffer
                    } else {
                        &mut self.input_buffer
                    };
                    let byte_position = buffer.chars().take(self.cursor_position).map(char::len_utf8).sum();
                    buffer.insert(byte_position, character);
                    self.cursor_position += 1;
                    return Action::None;
                }
                KeyCode::Backspace => {
                    let buffer = if self.task_schedule_focused {
                        &mut self.task_schedule_buffer
                    } else {
                        &mut self.input_buffer
                    };
                    if self.cursor_position > 0 {
                        let byte_position: usize = buffer.chars().take(self.cursor_position).map(char::len_utf8).sum();
                        let previous_width =
                            buffer.chars().nth(self.cursor_position - 1).map(char::len_utf8).unwrap_or(1);
                        buffer.remove(byte_position - previous_width);
                        self.cursor_position -= 1;
                    }
                    return Action::None;
                }
                KeyCode::Delete => {
                    let buffer = if self.task_schedule_focused {
                        &mut self.task_schedule_buffer
                    } else {
                        &mut self.input_buffer
                    };
                    if self.cursor_position < buffer.chars().count() {
                        let byte_position = buffer.chars().take(self.cursor_position).map(char::len_utf8).sum();
                        buffer.remove(byte_position);
                    }
                    return Action::None;
                }
                KeyCode::Left => {
                    self.cursor_position = self.cursor_position.saturating_sub(1);
                    return Action::None;
                }
                KeyCode::Right => {
                    let character_count = if self.task_schedule_focused {
                        self.task_schedule_buffer.chars().count()
                    } else {
                        self.input_buffer.chars().count()
                    };
                    if self.cursor_position < character_count {
                        self.cursor_position += 1;
                    }
                    return Action::None;
                }
                _ => {}
            }
        }

        match &self.dialog_type {
            Some(DialogType::TaskDetails { task }) => match key.code {
                KeyCode::Esc | KeyCode::Enter => Action::HideDialog,
                KeyCode::Char('e') => Action::ShowDialog(DialogType::TaskEdit {
                    task_uuid: task.uuid,
                    content: task.content.clone(),
                    project_uuid: task.project_uuid,
                }),
                KeyCode::Char('d') => Action::ShowDialog(DialogType::TaskDescriptionEdit {
                    task_uuid: task.uuid,
                    description: task.description.clone().unwrap_or_default(),
                }),
                KeyCode::Char('m') => Action::ShowDialog(DialogType::AiTaskManagement {
                    task: task.clone(),
                    stage: AiAssistStage::ContextEntry,
                    proposal: None,
                    report: None,
                }),
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_up();
                    Action::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_down();
                    Action::None
                }
                KeyCode::PageUp => {
                    self.page_up();
                    Action::None
                }
                KeyCode::PageDown => {
                    self.page_down();
                    Action::None
                }
                KeyCode::Home => {
                    self.scroll_to_top();
                    Action::None
                }
                KeyCode::End => {
                    self.scroll_to_bottom();
                    Action::None
                }
                _ => Action::None,
            },
            Some(DialogType::Info(_)) | Some(DialogType::Error(_)) => {
                // Info/error dialogs with scrolling support
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.scroll_up();
                        Action::None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down();
                        Action::None
                    }
                    KeyCode::PageUp => {
                        self.page_up();
                        Action::None
                    }
                    KeyCode::PageDown => {
                        self.page_down();
                        Action::None
                    }
                    KeyCode::Home => {
                        self.scroll_to_top();
                        Action::None
                    }
                    KeyCode::End => {
                        self.scroll_to_bottom();
                        Action::None
                    }
                    _ => Action::HideDialog, // Any other key dismisses the dialog
                }
            }
            Some(DialogType::Help) => {
                // Help dialog with scrolling support
                match key.code {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('h') => Action::HideDialog,
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.scroll_up();
                        Action::None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down();
                        Action::None
                    }
                    KeyCode::PageUp => {
                        self.page_up();
                        Action::None
                    }
                    KeyCode::PageDown => {
                        self.page_down();
                        Action::None
                    }
                    KeyCode::Home => {
                        self.scroll_to_top();
                        Action::None
                    }
                    KeyCode::End => {
                        self.scroll_to_bottom();
                        Action::None
                    }
                    _ => Action::None,
                }
            }
            Some(DialogType::Logs) => {
                // Logs dialog with scrolling support (same as help dialog)
                match key.code {
                    KeyCode::Esc | KeyCode::Char('G') | KeyCode::Char('q') => Action::HideDialog,
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.scroll_up();
                        Action::None
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down();
                        Action::None
                    }
                    KeyCode::PageUp => {
                        self.page_up();
                        Action::None
                    }
                    KeyCode::PageDown => {
                        self.page_down();
                        Action::None
                    }
                    KeyCode::Home => {
                        self.scroll_to_top();
                        Action::None
                    }
                    KeyCode::End => {
                        self.scroll_to_bottom();
                        Action::None
                    }
                    _ => Action::None,
                }
            }
            Some(DialogType::DeleteConfirmation { .. } | DialogType::EmptyTrashConfirmation { .. }) => match key.code {
                KeyCode::Esc => Action::HideDialog,
                KeyCode::Enter => self.handle_submit(),
                _ => Action::None,
            },
            Some(DialogType::TaskSearch) => match key.code {
                KeyCode::Esc => Action::HideDialog,
                KeyCode::Enter => Action::None,
                KeyCode::Down => {
                    if !self.search_results_focused {
                        if !self.search_results.is_empty() {
                            self.search_results_focused = true;
                            self.search_selected_index =
                                self.search_selected_index.min(self.search_results.len().saturating_sub(1));
                        }
                    } else if self.search_selected_index + 1 < self.search_results.len() {
                        self.search_selected_index += 1;
                    }
                    Action::None
                }
                KeyCode::Up => {
                    if self.search_results_focused {
                        if self.search_selected_index == 0 {
                            self.search_results_focused = false;
                        } else {
                            self.search_selected_index -= 1;
                        }
                    }
                    Action::None
                }
                KeyCode::Char('j') if self.search_results_focused => {
                    if self.search_selected_index + 1 < self.search_results.len() {
                        self.search_selected_index += 1;
                    }
                    Action::None
                }
                KeyCode::Char('k') if self.search_results_focused => {
                    if self.search_selected_index == 0 {
                        self.search_results_focused = false;
                    } else {
                        self.search_selected_index -= 1;
                    }
                    Action::None
                }
                KeyCode::Char('t') if self.search_results_focused => self
                    .search_results
                    .get(self.search_selected_index)
                    .map_or(Action::None, |task| Action::SetTaskDueToday(task.uuid)),
                KeyCode::Char(' ') if self.search_results_focused => self
                    .search_results
                    .get(self.search_selected_index)
                    .map_or(Action::None, Action::toggle_task),
                KeyCode::Char(c) if !self.search_results_focused => {
                    let byte_pos: usize = self
                        .input_buffer
                        .chars()
                        .take(self.cursor_position)
                        .map(|ch| ch.len_utf8())
                        .sum();
                    self.input_buffer.insert(byte_pos, c);
                    self.cursor_position += 1;
                    self.trigger_search()
                }
                KeyCode::Backspace if !self.search_results_focused => {
                    if self.cursor_position > 0 {
                        let byte_pos: usize = self
                            .input_buffer
                            .chars()
                            .take(self.cursor_position)
                            .map(|ch| ch.len_utf8())
                            .sum();
                        let prev_char_len = self
                            .input_buffer
                            .chars()
                            .nth(self.cursor_position - 1)
                            .map(|ch| ch.len_utf8())
                            .unwrap_or(1);
                        self.input_buffer.remove(byte_pos - prev_char_len);
                        self.cursor_position -= 1;
                        return self.trigger_search();
                    }
                    Action::None
                }
                KeyCode::Delete if !self.search_results_focused => {
                    let char_count = self.input_buffer.chars().count();
                    if self.cursor_position < char_count {
                        let byte_pos: usize = self
                            .input_buffer
                            .chars()
                            .take(self.cursor_position)
                            .map(|ch| ch.len_utf8())
                            .sum();
                        self.input_buffer.remove(byte_pos);
                        return self.trigger_search();
                    }
                    Action::None
                }
                KeyCode::Left if !self.search_results_focused => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    Action::None
                }
                KeyCode::Right if !self.search_results_focused => {
                    let char_count = self.input_buffer.chars().count();
                    if self.cursor_position < char_count {
                        self.cursor_position += 1;
                    }
                    Action::None
                }
                _ => Action::None,
            },
            _ => {
                // Input dialogs
                match key.code {
                    KeyCode::Esc => Action::HideDialog,
                    KeyCode::Enter => self.handle_submit(),
                    KeyCode::Char(c) => {
                        let byte_pos: usize = self
                            .input_buffer
                            .chars()
                            .take(self.cursor_position)
                            .map(|ch| ch.len_utf8())
                            .sum();
                        self.input_buffer.insert(byte_pos, c);
                        self.cursor_position += 1;
                        Action::None
                    }
                    KeyCode::Backspace => {
                        if self.cursor_position > 0 {
                            let byte_pos: usize = self
                                .input_buffer
                                .chars()
                                .take(self.cursor_position)
                                .map(|ch| ch.len_utf8())
                                .sum();
                            let prev_char_len = self
                                .input_buffer
                                .chars()
                                .nth(self.cursor_position - 1)
                                .map(|ch| ch.len_utf8())
                                .unwrap_or(1);
                            self.input_buffer.remove(byte_pos - prev_char_len);
                            self.cursor_position -= 1;
                        }
                        Action::None
                    }
                    KeyCode::Delete => {
                        let char_count = self.input_buffer.chars().count();
                        if self.cursor_position < char_count {
                            let byte_pos: usize = self
                                .input_buffer
                                .chars()
                                .take(self.cursor_position)
                                .map(|ch| ch.len_utf8())
                                .sum();
                            self.input_buffer.remove(byte_pos);
                        }
                        Action::None
                    }
                    KeyCode::Left => {
                        if self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                        Action::None
                    }
                    KeyCode::Right => {
                        let char_count = self.input_buffer.chars().count();
                        if self.cursor_position < char_count {
                            self.cursor_position += 1;
                        }
                        Action::None
                    }
                    KeyCode::Tab => {
                        if matches!(self.dialog_type, Some(DialogType::TaskCreation { .. })) {
                            let task_projects = self.get_task_projects();
                            if !task_projects.is_empty() {
                                // Clone needed data to avoid borrow issues
                                let projects_data: Vec<(Uuid, String)> =
                                    task_projects.iter().map(|p| (p.uuid, p.name.clone())).collect();

                                // Mark that user has explicitly selected a project via Tab
                                self.task_project_explicitly_selected = true;

                                self.selected_task_project_index = match self.selected_task_project_index {
                                    None => {
                                        // First tab: select first project
                                        self.selected_task_project_uuid = Some(projects_data[0].0);
                                        log::info!(
                                            "Tab: Selected project {} ({})",
                                            projects_data[0].1,
                                            projects_data[0].0
                                        );
                                        Some(0)
                                    }
                                    Some(index) => {
                                        let next_index = (index + 1) % (projects_data.len() + 1);
                                        if next_index == projects_data.len() {
                                            // Cycle back to "None" option (inbox)
                                            self.selected_task_project_uuid = None;
                                            log::info!("Tab: Selected inbox (no project)");
                                            None
                                        } else {
                                            // Select the project at next_index
                                            self.selected_task_project_uuid = Some(projects_data[next_index].0);
                                            log::info!(
                                                "Tab: Selected project {} ({})",
                                                projects_data[next_index].1,
                                                projects_data[next_index].0
                                            );
                                            Some(next_index)
                                        }
                                    }
                                };
                            }
                        } else if matches!(self.dialog_type, Some(DialogType::ProjectCreation)) {
                            let root_projects = self.get_root_projects();
                            if !root_projects.is_empty() {
                                self.selected_parent_project_index = match self.selected_parent_project_index {
                                    None => Some(0), // First tab: select first parent
                                    Some(index) => {
                                        let next_index = (index + 1) % (root_projects.len() + 1);
                                        if next_index == root_projects.len() {
                                            None // Cycle back to "None" option
                                        } else {
                                            Some(next_index)
                                        }
                                    }
                                };
                            }
                        }
                        Action::None
                    }
                    _ => Action::None,
                }
            }
        }
    }

    fn update(&mut self, action: Action) -> Action {
        match action {
            Action::AiProposalGenerated { task, proposal } => {
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.ai_selected_action_index = 0;
                self.ai_proposal_page = AiProposalPage::Recommendation;
                self.ai_recommendation_scroll = 0;
                self.ai_recommendation_max_scroll = 0;
                self.ai_error_message = None;
                self.dialog_type = Some(DialogType::AiTaskManagement {
                    task,
                    stage: AiAssistStage::ProposalReview,
                    proposal: Some(proposal),
                    report: None,
                });
                Action::None
            }
            Action::AiProposalFailed { task, message } => {
                self.ai_error_message = Some(message);
                self.dialog_type = Some(DialogType::AiTaskManagement {
                    task,
                    stage: AiAssistStage::ContextEntry,
                    proposal: None,
                    report: None,
                });
                Action::None
            }
            Action::AiProposalRevisionFailed {
                task,
                proposal,
                message,
            } => {
                self.ai_error_message = Some(message);
                self.ai_proposal_page = AiProposalPage::Actions;
                self.dialog_type = Some(DialogType::AiTaskManagement {
                    task,
                    stage: AiAssistStage::ProposalReview,
                    proposal: Some(proposal),
                    report: None,
                });
                Action::None
            }
            Action::AiAssistFinished { task, proposal, report } => {
                self.dialog_type = Some(DialogType::AiTaskManagement {
                    task,
                    stage: AiAssistStage::Result,
                    proposal: Some(proposal),
                    report: Some(report),
                });
                Action::None
            }
            Action::ShowDialog(dialog_type) => {
                // Check if this is a task creation dialog before moving the value
                let is_task_creation = matches!(dialog_type, DialogType::TaskCreation { .. });

                // Pre-populate input for edit dialogs
                match &dialog_type {
                    DialogType::TaskEdit { content, .. } => {
                        self.input_buffer = content.clone();
                        self.cursor_position = content.chars().count();
                    }
                    DialogType::TaskDescriptionEdit { description, .. } => {
                        self.input_buffer = description.clone();
                        self.cursor_position = description.chars().count();
                    }
                    DialogType::TaskTime { current_time, .. } => {
                        self.input_buffer = current_time.clone().unwrap_or_default();
                        self.cursor_position = self.input_buffer.chars().count();
                    }
                    DialogType::ProjectEdit { name, .. } => {
                        self.input_buffer = name.clone();
                        self.cursor_position = name.chars().count();
                    }
                    DialogType::LabelEdit { name, .. } => {
                        self.input_buffer = name.clone();
                        self.cursor_position = name.chars().count();
                    }
                    DialogType::TaskCreation {
                        default_project_uuid, ..
                    } => {
                        self.input_buffer.clear();
                        self.task_schedule_buffer.clear();
                        self.task_schedule_focused = false;
                        self.cursor_position = 0;
                        // Set the selected task project index and UUID if a default project is provided
                        if let Some(project_uuid) = default_project_uuid {
                            let task_projects = self.get_task_projects();
                            if let Some(index) = task_projects.iter().position(|p| &p.uuid == project_uuid) {
                                self.selected_task_project_index = Some(index);
                                self.selected_task_project_uuid = Some(*project_uuid);
                                let proj_name = self
                                    .projects
                                    .iter()
                                    .find(|p| &p.uuid == project_uuid)
                                    .map(|p| p.name.as_str())
                                    .unwrap_or("unknown");
                                log::info!("Dialog opened with default project: {} ({})", proj_name, project_uuid);
                            }
                        } else {
                            log::info!("Dialog opened with no default project (inbox)");
                        }
                    }
                    DialogType::TaskSearch => {
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.search_results.clear();
                        self.search_selected_index = 0;
                        self.search_results_focused = false;
                    }
                    DialogType::AiTaskManagement { .. } => {
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.ai_selected_action_index = 0;
                        self.ai_error_message = None;
                    }
                    _ => {
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                    }
                }
                self.dialog_type = Some(dialog_type.clone());
                // Only reset project index for non-task-creation dialogs
                if !is_task_creation {
                    self.selected_project_index = 0;
                }

                // Trigger initial search for TaskSearch dialog
                if matches!(dialog_type, DialogType::TaskSearch) {
                    return self.trigger_search();
                }

                Action::None
            }
            Action::HideDialog => {
                self.clear_dialog();
                Action::None
            }
            _ => action,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        if let Some(dialog_type) = self.dialog_type.clone() {
            match dialog_type {
                DialogType::TaskDetails { task } => {
                    let project_name = self
                        .projects
                        .iter()
                        .find(|project| project.uuid == task.project_uuid)
                        .map(|project| project.name.as_str())
                        .unwrap_or("Unknown");
                    task_dialogs::render_task_details_dialog(
                        f,
                        rect,
                        &task,
                        project_name,
                        self.scroll_offset,
                        &mut self.scrollbar_state,
                    );
                }
                DialogType::TaskCreation { .. } => self.render_task_creation_dialog(f, rect),
                DialogType::TaskEdit { .. } => self.render_task_edit_dialog(f, rect),
                DialogType::TaskDescriptionEdit { .. } => {
                    task_dialogs::render_task_description_edit_dialog(f, rect, &self.input_buffer, self.cursor_position)
                }
                DialogType::TaskTime { .. } => {
                    task_dialogs::render_task_time_dialog(f, rect, &self.input_buffer, self.cursor_position)
                }
                DialogType::AiTaskManagement {
                    task,
                    stage,
                    proposal,
                    report,
                } => {
                    self.render_ai_task_management_dialog(f, rect, &task, stage, proposal.as_ref(), report.as_ref());
                }
                DialogType::ProjectCreation => {
                    self.render_project_creation_dialog(f, rect);
                }
                DialogType::ProjectEdit { .. } => {
                    self.render_project_edit_dialog(f, rect);
                }
                DialogType::LabelCreation => {
                    self.render_label_creation_dialog(f, rect);
                }
                DialogType::LabelEdit { .. } => {
                    self.render_label_edit_dialog(f, rect);
                }
                DialogType::DeleteConfirmation { item_type, .. } => {
                    self.render_delete_confirmation_dialog(f, rect, &item_type);
                }
                DialogType::EmptyTrashConfirmation { count } => {
                    self.render_delete_confirmation_dialog(f, rect, &format!("{} tasks from Trash", count));
                }
                DialogType::Info(message) => {
                    self.render_info_dialog(f, rect, &message);
                }
                DialogType::Error(message) => {
                    self.render_error_dialog(f, rect, &message);
                }
                DialogType::Help => {
                    self.render_help_dialog(f, rect);
                }
                DialogType::Logs => {
                    self.render_logs_dialog(f, rect);
                }
                DialogType::TaskSearch => {
                    self.render_task_search_dialog(f, rect);
                }
            }
        }
    }
}
