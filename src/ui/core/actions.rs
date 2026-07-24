use crate::{entities::task, sync::SyncStatus};
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
    TaskTime {
        task_uuid: Uuid,
        current_time: Option<String>,
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
