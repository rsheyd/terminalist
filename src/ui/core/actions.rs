use crate::{entities::task, priority::TaskPriority, sync::SyncStatus};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct NavigationCounts {
    pub today: usize,
    pub tomorrow: usize,
    pub upcoming: usize,
    pub trash: usize,
    pub projects: HashMap<Uuid, usize>,
    pub labels: HashMap<Uuid, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskDueDate {
    None,
    Today,
    Tomorrow,
    NextWeek,
    Weekend,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AiAssistStage {
    ContextEntry,
    RevisionEntry,
    Generating,
    ProposalReview,
    ApplyConfirmation,
    Applying,
    Result,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AiProposalPage {
    #[default]
    Recommendation,
    Actions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiTaskProposal {
    pub summary: String,
    pub actions: Vec<AiProposedAction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiProposedAction {
    pub enabled: bool,
    pub kind: AiProposedActionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AiProjectDestination {
    Existing { uuid: Uuid, name: String },
    Proposed { reference: String, name: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AiProposedActionKind {
    AddCompletionNote {
        content: String,
    },
    CreateProject {
        reference: String,
        name: String,
    },
    CreateTask {
        content: String,
        description: String,
        destination: AiProjectDestination,
        priority: i32,
    },
    MoveOriginalTask {
        destination: AiProjectDestination,
    },
    CompleteTask,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiExecutionReport {
    pub completed_actions: Vec<String>,
    pub failure: Option<String>,
}

impl AiExecutionReport {
    pub fn success(completed_actions: Vec<String>) -> Self {
        Self {
            completed_actions,
            failure: None,
        }
    }

    pub fn failed(completed_actions: Vec<String>, failure: String) -> Self {
        Self {
            completed_actions,
            failure: Some(failure),
        }
    }
}

impl AiProposedAction {
    pub fn description(&self) -> String {
        match &self.kind {
            AiProposedActionKind::AddCompletionNote { content } => {
                format!("Add completion note: {content}")
            }
            AiProposedActionKind::CreateProject { name, .. } => {
                format!("Create project: {name}")
            }
            AiProposedActionKind::CreateTask {
                content,
                description,
                destination,
                priority,
            } => {
                let project_name = match destination {
                    AiProjectDestination::Existing { name, .. } | AiProjectDestination::Proposed { name, .. } => name,
                };
                let priority = TaskPriority::from_todoist(*priority).label();
                format!("Create undated {priority}-priority task in {project_name}: {content}\n  {description}")
            }
            AiProposedActionKind::MoveOriginalTask { destination } => {
                let project_name = match destination {
                    AiProjectDestination::Existing { name, .. } | AiProjectDestination::Proposed { name, .. } => name,
                };
                format!("Move the original task to {project_name}")
            }
            AiProposedActionKind::CompleteTask => "Complete the original task".to_string(),
        }
    }
}

impl AiTaskProposal {
    /// Deterministic proposal used by the interaction prototype.
    ///
    /// Increment 3 will replace this with a validated provider response.
    pub fn mock_for(task: &task::Model, context: &str, project_uuid: Option<Uuid>) -> Self {
        let context = context.trim();
        let completion_note = if context.is_empty() {
            "Reviewed the task and preserved the current outcome.".to_string()
        } else {
            format!("Outcome note: {context}")
        };

        Self {
            summary: "The original task may be substantially complete. Preserve its outcome and move optional future work out of active planning.".to_string(),
            actions: vec![
                AiProposedAction {
                    enabled: true,
                    kind: AiProposedActionKind::AddCompletionNote {
                        content: completion_note,
                    },
                },
                AiProposedAction {
                    enabled: project_uuid.is_some(),
                    kind: AiProposedActionKind::CreateTask {
                        content: format!("Revisit: {}", task.content),
                        description:
                            "Revisit if the need recurs, a better option appears, or discretionary project time becomes available."
                                .to_string(),
                        destination: AiProjectDestination::Existing {
                            uuid: project_uuid.unwrap_or_else(Uuid::nil),
                            name: "Follow-up".to_string(),
                        },
                        priority: 1,
                    },
                },
                AiProposedAction {
                    enabled: true,
                    kind: AiProposedActionKind::CompleteTask,
                },
            ],
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let enabled = self.actions.iter().filter(|action| action.enabled).collect::<Vec<_>>();
        if enabled.is_empty() {
            return Err("Select at least one proposed action.".to_string());
        }
        if enabled.len() > 8 {
            return Err("A proposal can apply at most eight actions.".to_string());
        }

        for action in enabled {
            match &action.kind {
                AiProposedActionKind::AddCompletionNote { content } if content.trim().is_empty() => {
                    return Err("Completion notes cannot be empty.".to_string());
                }
                AiProposedActionKind::CreateProject { reference, name } => {
                    if reference.trim().is_empty() || name.trim().is_empty() {
                        return Err("Created projects require a reference and name.".to_string());
                    }
                }
                AiProposedActionKind::CreateTask {
                    content,
                    description,
                    destination,
                    priority,
                } => {
                    if content.trim().is_empty() || description.trim().is_empty() {
                        return Err("Created tasks require a title and description.".to_string());
                    }
                    if !(1..=4).contains(priority) {
                        return Err("Todoist priorities must be between 1 and 4.".to_string());
                    }
                    self.validate_destination(destination)?;
                }
                AiProposedActionKind::MoveOriginalTask { destination } => {
                    self.validate_destination(destination)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn validate_destination(&self, destination: &AiProjectDestination) -> Result<(), String> {
        match destination {
            AiProjectDestination::Existing { uuid, name } => {
                if uuid.is_nil() || name.trim().is_empty() {
                    return Err("Existing project destinations require an ID and name.".to_string());
                }
            }
            AiProjectDestination::Proposed { reference, name } => {
                let exists = self.actions.iter().any(|action| {
                    action.enabled
                        && matches!(
                            &action.kind,
                            AiProposedActionKind::CreateProject {
                                reference: candidate,
                                name: candidate_name,
                            } if candidate == reference && candidate_name == name
                        )
                });
                if !exists {
                    return Err(format!(
                        "Project destination '{name}' has no enabled create-project action."
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn enabled_actions_in_execution_order(&self) -> Vec<&AiProposedAction> {
        let mut actions = self.actions.iter().filter(|action| action.enabled).collect::<Vec<_>>();
        actions.sort_by_key(|action| match action.kind {
            AiProposedActionKind::CreateProject { .. } => 0,
            AiProposedActionKind::AddCompletionNote { .. } => 1,
            AiProposedActionKind::CreateTask { .. } | AiProposedActionKind::MoveOriginalTask { .. } => 2,
            AiProposedActionKind::CompleteTask => 3,
        });
        actions
    }
}

/// Represents the currently selected item in the sidebar
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SidebarSelection {
    #[default]
    Today, // Today view (special view)
    Agenda,   // Local smart view derived from Today
    Tomorrow, // Tomorrow view (special view)
    Upcoming, // Upcoming view (tasks with future due dates)
    Trash,
    Label(Uuid),
    Project(Uuid),
}

#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    NavigateToSidebar(SidebarSelection),
    NextTask,
    PreviousTask,

    // Task operations
    ToggleTasks(Vec<(Uuid, bool)>),
    DeleteTask(Uuid),
    CyclePriority(Uuid),
    SetTaskDueToday(Uuid),
    SetTaskDueTomorrow(Uuid),
    SetTaskDueNextWeek(Uuid),
    SetTaskDueWeekEnd(Uuid),
    SetTaskDueTime {
        task_uuid: Uuid,
        due_datetime: String,
    },
    SetTasksDueDate {
        task_ids: Vec<Uuid>,
        due_date: TaskDueDate,
    },
    CreateTask {
        content: String,
        project_uuid: Option<Uuid>,
        due_date: Option<String>,
        label_uuid: Option<Uuid>,
    },
    EditTask {
        task_uuid: Uuid,
        content: String,
    },
    EditTaskDescription {
        task_uuid: Uuid,
        description: String,
    },
    AiAssist {
        task: Box<task::Model>,
        context: String,
    },
    AiReviseProposal {
        task: Box<task::Model>,
        original_context: String,
        proposal: AiTaskProposal,
        revision: String,
    },
    AiProposalGenerated {
        task: Box<task::Model>,
        proposal: AiTaskProposal,
    },
    AiProposalFailed {
        task: Box<task::Model>,
        message: String,
    },
    AiProposalRevisionFailed {
        task: Box<task::Model>,
        proposal: AiTaskProposal,
        message: String,
    },
    AiApplyProposal {
        task: Box<task::Model>,
        proposal: AiTaskProposal,
    },
    AiAssistFinished {
        task: Box<task::Model>,
        proposal: AiTaskProposal,
        report: AiExecutionReport,
    },
    RestoreTask(Uuid),
    EmptyTrash,

    // Project operations
    CreateProject {
        name: String,
        parent_uuid: Option<Uuid>,
    },
    EditProject {
        project_uuid: Uuid,
        name: String,
    },
    DeleteProject(Uuid),

    // Label operations
    CreateLabel {
        name: String,
    },
    EditLabel {
        label_uuid: Uuid,
        name: String,
    },
    DeleteLabel(Uuid),

    // Sync operations
    StartSync,
    RefreshLocalData, // Debug mode: refresh from local DB without API sync
    SyncCompleted(SyncStatus),
    SyncFailed(String),
    DataLoaded(Box<crate::ui::core::ViewSnapshot>),
    DataLoadFailed {
        generation: u64,
        selection: SidebarSelection,
        message: String,
    },
    SearchTasks(String), // Query for task search
    SearchResultsLoaded {
        query: String,
        results: Vec<crate::entities::task::Model>,
    },

    // Data refresh after task operations
    RefreshData,

    // UI operations
    ToggleSidebar,
    ShowHelp(bool),
    ShowDebug(bool),
    ShowDialog(DialogType),
    HideDialog,
    HelpScrollUp,
    HelpScrollDown,
    HelpScrollToTop,
    HelpScrollToBottom,
    Consumed,

    // App control
    Quit,
    None,
}

impl Action {
    /// Whether this action starts or confirms a remote/local data mutation.
    ///
    /// Blocking background work may temporarily make the displayed snapshot stale. Read-only
    /// actions remain safe during that window, but mutations wait until the latest snapshot has
    /// been applied.
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Self::ToggleTasks(_)
                | Self::DeleteTask(_)
                | Self::CyclePriority(_)
                | Self::SetTaskDueToday(_)
                | Self::SetTaskDueTomorrow(_)
                | Self::SetTaskDueNextWeek(_)
                | Self::SetTaskDueWeekEnd(_)
                | Self::SetTaskDueTime { .. }
                | Self::SetTasksDueDate { .. }
                | Self::CreateTask { .. }
                | Self::EditTask { .. }
                | Self::EditTaskDescription { .. }
                | Self::AiApplyProposal { .. }
                | Self::RestoreTask(_)
                | Self::EmptyTrash
                | Self::CreateProject { .. }
                | Self::EditProject { .. }
                | Self::DeleteProject(_)
                | Self::CreateLabel { .. }
                | Self::EditLabel { .. }
                | Self::DeleteLabel(_)
                | Self::ShowDialog(
                    DialogType::TaskCreation { .. }
                        | DialogType::TaskEdit { .. }
                        | DialogType::TaskDescriptionEdit { .. }
                        | DialogType::TaskTime { .. }
                        | DialogType::ProjectCreation
                        | DialogType::ProjectEdit { .. }
                        | DialogType::LabelCreation
                        | DialogType::LabelEdit { .. }
                        | DialogType::DeleteConfirmation { .. }
                        | DialogType::EmptyTrashConfirmation { .. }
                )
        )
    }

    /// Build the shared completion-toggle action used by every task view.
    ///
    /// The boolean records whether the task should be restored rather than
    /// completed. Deleted tasks use the same restore path as completed tasks.
    pub fn toggle_tasks<'a>(tasks: impl IntoIterator<Item = &'a task::Model>) -> Self {
        let tasks = tasks
            .into_iter()
            .map(|task| (task.uuid, task.is_deleted || task.is_completed))
            .collect::<Vec<_>>();

        if tasks.is_empty() {
            Self::None
        } else {
            Self::ToggleTasks(tasks)
        }
    }

    pub fn toggle_task(task: &task::Model) -> Self {
        Self::toggle_tasks([task])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_classification_keeps_read_only_actions_available() {
        assert!(!Action::NextTask.is_mutation());
        assert!(!Action::NavigateToSidebar(SidebarSelection::Today).is_mutation());
        assert!(!Action::SearchTasks("needle".to_string()).is_mutation());
        assert!(!Action::ShowDialog(DialogType::Help).is_mutation());
    }

    #[test]
    fn mutation_classification_includes_edit_entry_points_and_confirmations() {
        assert!(Action::CreateTask {
            content: "New task".to_string(),
            project_uuid: None,
            due_date: None,
            label_uuid: None,
        }
        .is_mutation());
        assert!(Action::ShowDialog(DialogType::TaskCreation {
            default_project_uuid: None,
            default_due_date: None,
            default_label_uuid: None,
        })
        .is_mutation());
        assert!(Action::ShowDialog(DialogType::DeleteConfirmation {
            item_type: "task".to_string(),
            item_uuid: Uuid::new_v4(),
        })
        .is_mutation());

        let task = task::Model {
            uuid: Uuid::new_v4(),
            backend_uuid: Uuid::new_v4(),
            remote_id: "remote".to_string(),
            content: "Example".to_string(),
            description: None,
            project_uuid: Uuid::new_v4(),
            section_uuid: None,
            parent_uuid: None,
            priority: 1,
            order_index: 0,
            due_date: None,
            due_datetime: None,
            is_recurring: false,
            deadline: None,
            duration: None,
            is_completed: false,
            completed_at: None,
            is_deleted: false,
            deleted_at: None,
        };
        let proposal = AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
        assert!(Action::AiApplyProposal {
            task: Box::new(task),
            proposal,
        }
        .is_mutation());
    }

    #[test]
    fn ai_proposal_validates_destination_and_forces_completion_last() {
        let task = task::Model {
            uuid: Uuid::new_v4(),
            backend_uuid: Uuid::new_v4(),
            remote_id: "remote".to_string(),
            content: "Example".to_string(),
            description: None,
            project_uuid: Uuid::new_v4(),
            section_uuid: None,
            parent_uuid: None,
            priority: 1,
            order_index: 0,
            due_date: None,
            due_datetime: None,
            is_recurring: false,
            deadline: None,
            duration: None,
            is_completed: false,
            completed_at: None,
            is_deleted: false,
            deleted_at: None,
        };
        let missing_destination = AiTaskProposal::mock_for(&task, "Handled", None);
        assert!(missing_destination.validate().is_ok());

        let mut proposal = AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
        proposal.actions.reverse();
        assert!(proposal.validate().is_ok());
        assert!(matches!(
            proposal.enabled_actions_in_execution_order().last().map(|action| &action.kind),
            Some(AiProposedActionKind::CompleteTask)
        ));
    }
}

#[derive(Debug, Clone)]
pub enum DialogType {
    TaskDetails {
        task: Box<task::Model>,
    },
    TaskCreation {
        default_project_uuid: Option<Uuid>,
        default_due_date: Option<String>,
        default_label_uuid: Option<Uuid>,
    },
    TaskEdit {
        task_uuid: Uuid,
        content: String,
        project_uuid: Uuid,
    },
    TaskDescriptionEdit {
        task_uuid: Uuid,
        description: String,
    },
    TaskTime {
        task_uuid: Uuid,
        current_time: Option<String>,
    },
    AiTaskManagement {
        task: Box<task::Model>,
        stage: AiAssistStage,
        proposal: Option<AiTaskProposal>,
        report: Option<AiExecutionReport>,
    },
    ProjectCreation,
    ProjectEdit {
        project_uuid: Uuid,
        name: String,
    },
    LabelCreation,
    LabelEdit {
        label_uuid: Uuid,
        name: String,
    },
    DeleteConfirmation {
        item_type: String,
        item_uuid: Uuid,
    },
    EmptyTrashConfirmation {
        count: usize,
    },
    Error(String),
    Info(String),
    Help,
    Logs,
    TaskSearch,
}
