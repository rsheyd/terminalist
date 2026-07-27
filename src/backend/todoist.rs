//! Todoist backend implementation.

use super::{
    Backend, BackendError, BackendLabel, BackendProject, BackendSection, BackendSyncData, BackendTask, CreateLabelArgs,
    CreateProjectArgs, CreateTaskArgs, UpdateLabelArgs, UpdateProjectArgs, UpdateTaskArgs,
};
use crate::todoist::TodoistWrapper;
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Deserialize)]
struct CompletedTasksPage {
    #[serde(alias = "results")]
    items: Vec<crate::todoist::Task>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    sync_token: String,
    #[serde(default)]
    full_sync: bool,
    #[serde(default)]
    projects: Vec<SyncProject>,
    #[serde(default, alias = "tasks")]
    items: Vec<SyncTask>,
    #[serde(default)]
    labels: Vec<SyncLabel>,
    #[serde(default)]
    sections: Vec<SyncSection>,
}

#[derive(Debug, Deserialize)]
struct SyncProject {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    is_favorite: bool,
    #[serde(default, alias = "is_inbox_project")]
    inbox_project: bool,
    #[serde(default)]
    child_order: i32,
    parent_id: Option<String>,
    #[serde(default)]
    is_deleted: bool,
}

#[derive(Debug, Deserialize)]
struct SyncTask {
    id: String,
    #[serde(default)]
    content: String,
    description: Option<String>,
    project_id: Option<String>,
    section_id: Option<String>,
    parent_id: Option<String>,
    #[serde(default)]
    priority: i32,
    #[serde(default)]
    child_order: i32,
    due: Option<crate::todoist::Due>,
    deadline: Option<crate::todoist::Deadline>,
    duration: Option<crate::todoist::Duration>,
    #[serde(default)]
    checked: bool,
    completed_at: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    is_deleted: bool,
}

#[derive(Debug, Deserialize)]
struct SyncLabel {
    id: String,
    #[serde(default)]
    name: String,
    order: Option<i32>,
    #[serde(default)]
    is_favorite: bool,
    #[serde(default)]
    is_deleted: bool,
}

#[derive(Debug, Deserialize)]
struct SyncSection {
    id: String,
    #[serde(default)]
    name: String,
    project_id: Option<String>,
    #[serde(default)]
    section_order: i32,
    #[serde(default)]
    is_deleted: bool,
}

/// Todoist backend implementation.
pub struct TodoistBackend {
    wrapper: TodoistWrapper,
    api_token: String,
    client: reqwest::Client,
}

impl TodoistBackend {
    /// Create a new Todoist backend with the provided API token.
    pub fn new(api_token: String) -> Self {
        Self {
            wrapper: TodoistWrapper::new(api_token.clone()),
            api_token,
            client: reqwest::Client::new(),
        }
    }

    // Helper: Transform Todoist API project → Backend project
    fn project_to_backend(api_project: &crate::todoist::Project) -> BackendProject {
        BackendProject {
            remote_id: api_project.id.clone(),
            name: api_project.name.clone(),
            is_favorite: api_project.is_favorite,
            is_inbox: api_project.inbox_project,
            order_index: 0, // order field removed from API v1
            parent_remote_id: api_project.parent_id.clone(),
        }
    }

    // Helper: Transform Todoist API task → Backend task
    fn task_to_backend(api_task: &crate::todoist::Task) -> BackendTask {
        let due_datetime = api_task.due.as_ref().and_then(|due| {
            due.datetime
                .clone()
                .or_else(|| due.date.contains('T').then(|| due.date.clone()))
        });
        let due_date = api_task
            .due
            .as_ref()
            .and_then(|due| due.date.split('T').next().map(str::to_string));
        BackendTask {
            remote_id: api_task.id.clone(),
            content: api_task.content.clone(),
            description: Some(api_task.description.clone()),
            project_remote_id: api_task.project_id.clone(),
            section_remote_id: api_task.section_id.clone(),
            parent_remote_id: api_task.parent_id.clone(),
            priority: api_task.priority,
            order_index: 0, // order field removed from API v1
            due_date,
            due_datetime,
            is_recurring: api_task.due.as_ref().map(|d| d.is_recurring).unwrap_or(false),
            deadline: None, // Todoist doesn't have deadline
            duration: api_task.duration.as_ref().map(|d| format!("{} {}", d.amount, d.unit)),
            is_completed: api_task.checked || api_task.completed_at.is_some(),
            completed_at: api_task.completed_at.clone(),
            labels: api_task.labels.clone(),
        }
    }

    // Helper: Transform Todoist API label → Backend label
    fn label_to_backend(api_label: &crate::todoist::Label) -> BackendLabel {
        BackendLabel {
            remote_id: api_label.id.clone(),
            name: api_label.name.clone(),
            order_index: api_label.order.unwrap_or(0),
            is_favorite: api_label.is_favorite,
        }
    }

    // Helper: Transform Todoist API section → Backend section
    fn section_to_backend(api_section: &crate::todoist::Section) -> BackendSection {
        BackendSection {
            remote_id: api_section.id.clone(),
            name: api_section.name.clone(),
            project_remote_id: api_section.project_id.clone(),
            order_index: api_section.section_order,
        }
    }

    fn task_create_args_to_todoist(args: CreateTaskArgs) -> crate::todoist::CreateTaskArgs {
        crate::todoist::CreateTaskArgs {
            content: args.content,
            description: args.description,
            project_id: args.project_remote_id,
            section_id: args.section_remote_id,
            parent_id: args.parent_remote_id,
            priority: args.priority,
            due_date: args.due_date,
            due_datetime: args.due_datetime,
            labels: Some(args.labels),
            duration: args
                .duration
                .as_deref()
                .and_then(|duration| duration.split_whitespace().next())
                .and_then(|amount| amount.parse().ok()),
            ..Default::default()
        }
    }

    fn task_update_args_to_todoist(args: &UpdateTaskArgs) -> crate::todoist::UpdateTaskArgs {
        crate::todoist::UpdateTaskArgs {
            content: args.content.clone(),
            description: args.description.clone(),
            priority: args.priority,
            due_string: args.clear_due_date.then(|| "no date".to_string()),
            due_date: args.due_date.clone(),
            due_datetime: args.due_datetime.clone(),
            labels: args.labels.clone(),
            duration: args
                .duration
                .as_deref()
                .and_then(|duration| duration.split_whitespace().next())
                .and_then(|amount| amount.parse().ok()),
            ..Default::default()
        }
    }

    fn sync_task_to_backend(task: SyncTask) -> Result<BackendTask, BackendError> {
        let project_remote_id = task
            .project_id
            .ok_or_else(|| BackendError::InvalidData(format!("Task {} has no project_id", task.id)))?;
        let due_datetime = task.due.as_ref().and_then(|due| {
            due.datetime
                .clone()
                .or_else(|| due.date.contains('T').then(|| due.date.clone()))
        });
        let due_date = task.due.as_ref().and_then(|due| due.date.split('T').next().map(str::to_string));

        Ok(BackendTask {
            remote_id: task.id,
            content: task.content,
            description: task.description,
            project_remote_id,
            section_remote_id: task.section_id,
            parent_remote_id: task.parent_id,
            priority: task.priority,
            order_index: task.child_order,
            due_date,
            due_datetime,
            is_recurring: task.due.as_ref().map(|due| due.is_recurring).unwrap_or(false),
            deadline: task.deadline.map(|deadline| deadline.date),
            duration: task.duration.map(|duration| format!("{} {}", duration.amount, duration.unit)),
            is_completed: task.checked || task.completed_at.is_some(),
            completed_at: task.completed_at,
            labels: task.labels,
        })
    }
}

#[async_trait]
impl Backend for TodoistBackend {
    fn backend_type(&self) -> &str {
        "todoist"
    }

    async fn fetch_projects(&self) -> Result<Vec<BackendProject>, BackendError> {
        let mut all_projects = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_projects(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_projects.extend(response.results.iter().map(Self::project_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_projects)
    }

    async fn fetch_tasks(&self) -> Result<Vec<BackendTask>, BackendError> {
        let mut all_tasks = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_tasks(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_tasks.extend(response.results.iter().map(Self::task_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_tasks)
    }

    async fn fetch_completed_tasks(&self, since: &str, until: &str) -> Result<Vec<BackendTask>, BackendError> {
        let mut all_tasks = Vec::new();
        let mut cursor = None;

        loop {
            let mut request = self
                .client
                .get("https://api.todoist.com/api/v1/tasks/completed/by_completion_date")
                .bearer_auth(&self.api_token)
                .query(&[("since", since), ("until", until), ("limit", "200")]);
            if let Some(cursor) = &cursor {
                request = request.query(&[("cursor", cursor)]);
            }
            let response = request.send().await.map_err(|e| BackendError::Network(e.to_string()))?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(BackendError::Other(format!("Todoist returned {status}: {body}")));
            }
            let response = response
                .json::<CompletedTasksPage>()
                .await
                .map_err(|e| BackendError::InvalidData(e.to_string()))?;

            all_tasks.extend(response.items.iter().map(Self::task_to_backend));
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_tasks)
    }

    async fn fetch_labels(&self) -> Result<Vec<BackendLabel>, BackendError> {
        let mut all_labels = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_labels(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_labels.extend(response.results.iter().map(Self::label_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_labels)
    }

    async fn fetch_sections(&self) -> Result<Vec<BackendSection>, BackendError> {
        let mut all_sections = Vec::new();
        let mut cursor: Option<String> = None;

        // Fetch all pages with limit=200
        loop {
            let response = self
                .wrapper
                .get_sections(Some(200), cursor.clone())
                .await
                .map_err(|e| BackendError::Network(e.to_string()))?;

            all_sections.extend(response.results.iter().map(Self::section_to_backend));

            // Check if there are more pages
            if response.next_cursor.is_none() {
                break;
            }
            cursor = response.next_cursor;
        }

        Ok(all_sections)
    }

    async fn fetch_sync(&self, sync_token: Option<&str>) -> Result<BackendSyncData, BackendError> {
        let response = self
            .client
            .post("https://api.todoist.com/api/v1/sync")
            .bearer_auth(&self.api_token)
            .form(&[
                ("sync_token", sync_token.unwrap_or("*")),
                ("resource_types", r#"["projects","items","labels","sections"]"#),
            ])
            .send()
            .await
            .map_err(|error| BackendError::Network(error.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::BAD_REQUEST && body.to_ascii_lowercase().contains("sync_token") {
                return Err(BackendError::InvalidSyncToken(body));
            }
            return Err(BackendError::Other(format!("Todoist returned {status}: {body}")));
        }

        let response = response
            .json::<SyncResponse>()
            .await
            .map_err(|error| BackendError::InvalidData(error.to_string()))?;
        let mut data = BackendSyncData {
            full_sync: response.full_sync,
            sync_token: Some(response.sync_token),
            ..BackendSyncData::default()
        };

        for project in response.projects {
            if project.is_deleted {
                data.deleted_project_ids.push(project.id);
            } else {
                data.projects.push(BackendProject {
                    remote_id: project.id,
                    name: project.name,
                    is_favorite: project.is_favorite,
                    is_inbox: project.inbox_project,
                    order_index: project.child_order,
                    parent_remote_id: project.parent_id,
                });
            }
        }
        for task in response.items {
            if task.is_deleted {
                data.deleted_task_ids.push(task.id);
            } else {
                data.tasks.push(Self::sync_task_to_backend(task)?);
            }
        }
        for label in response.labels {
            if label.is_deleted {
                data.deleted_label_ids.push(label.id);
            } else {
                data.labels.push(BackendLabel {
                    remote_id: label.id,
                    name: label.name,
                    order_index: label.order.unwrap_or(0),
                    is_favorite: label.is_favorite,
                });
            }
        }
        for section in response.sections {
            if section.is_deleted {
                data.deleted_section_ids.push(section.id);
            } else {
                let project_remote_id = section
                    .project_id
                    .ok_or_else(|| BackendError::InvalidData(format!("Section {} has no project_id", section.id)))?;
                data.sections.push(BackendSection {
                    remote_id: section.id,
                    name: section.name,
                    project_remote_id,
                    order_index: section.section_order,
                });
            }
        }

        if data.full_sync {
            let (completed_since, completed_until) = crate::utils::datetime::today_completion_range();
            let completed = self.fetch_completed_tasks(&completed_since, &completed_until).await?;
            data.tasks = completed.into_iter().chain(data.tasks).collect();
        }

        Ok(data)
    }

    async fn create_project(&self, args: CreateProjectArgs) -> Result<BackendProject, BackendError> {
        let todoist_args = crate::todoist::CreateProjectArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            parent_id: args.parent_remote_id,
            view_style: None,
        };

        let project = self
            .wrapper
            .create_project(&todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::project_to_backend(&project))
    }

    async fn update_project(&self, remote_id: &str, args: UpdateProjectArgs) -> Result<BackendProject, BackendError> {
        let todoist_args = crate::todoist::UpdateProjectArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            view_style: None,
        };

        let project = self
            .wrapper
            .update_project(remote_id, &todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::project_to_backend(&project))
    }

    async fn delete_project(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .delete_project(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn create_task(&self, args: CreateTaskArgs) -> Result<BackendTask, BackendError> {
        let todoist_args = Self::task_create_args_to_todoist(args);

        let task = self
            .wrapper
            .create_task(&todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::task_to_backend(&task))
    }

    async fn update_task(&self, remote_id: &str, args: UpdateTaskArgs) -> Result<BackendTask, BackendError> {
        let move_project_id = args.project_remote_id.clone();

        let todoist_args = Self::task_update_args_to_todoist(&args);

        let updated_task = if todoist_args.has_updates() {
            Some(
                self.wrapper
                    .update_task(remote_id, &todoist_args)
                    .await
                    .map_err(|e| BackendError::Network(e.to_string()))?,
            )
        } else {
            None
        };

        let task = if let Some(project_id) = move_project_id {
            let response = self
                .client
                .post(format!("https://api.todoist.com/api/v1/tasks/{remote_id}/move"))
                .bearer_auth(&self.api_token)
                .json(&serde_json::json!({ "project_id": project_id }))
                .send()
                .await
                .map_err(|error| BackendError::Network(error.to_string()))?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(BackendError::Other(format!("Todoist returned {status}: {body}")));
            }
            response
                .json::<crate::todoist::Task>()
                .await
                .map_err(|error| BackendError::InvalidData(error.to_string()))?
        } else {
            updated_task.ok_or_else(|| BackendError::InvalidData("No task updates specified".to_string()))?
        };

        Ok(Self::task_to_backend(&task))
    }

    async fn delete_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .delete_task(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn complete_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .complete_task(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn create_task_comment(&self, remote_task_id: &str, content: &str) -> Result<(), BackendError> {
        let args = crate::todoist::CreateCommentArgs {
            content: content.to_string(),
            task_id: Some(remote_task_id.to_string()),
            ..Default::default()
        };
        self.wrapper
            .create_comment(&args)
            .await
            .map(|_| ())
            .map_err(|error| BackendError::Network(error.to_string()))
    }

    async fn reopen_task(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .reopen_task(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn create_label(&self, args: CreateLabelArgs) -> Result<BackendLabel, BackendError> {
        let todoist_args = crate::todoist::CreateLabelArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            ..Default::default()
        };

        let label = self
            .wrapper
            .create_label(&todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::label_to_backend(&label))
    }

    async fn update_label(&self, remote_id: &str, args: UpdateLabelArgs) -> Result<BackendLabel, BackendError> {
        let todoist_args = crate::todoist::UpdateLabelArgs {
            name: args.name,
            color: None,
            is_favorite: args.is_favorite,
            ..Default::default()
        };

        let label = self
            .wrapper
            .update_label(remote_id, &todoist_args)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(Self::label_to_backend(&label))
    }

    async fn delete_label(&self, remote_id: &str) -> Result<(), BackendError> {
        self.wrapper
            .delete_label(remote_id)
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_an_inbox_task_omits_the_project_id() {
        let args = CreateTaskArgs {
            content: "Inbox task".to_string(),
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: None,
            due_datetime: None,
            duration: None,
            labels: Vec::new(),
        };

        let todoist_args = TodoistBackend::task_create_args_to_todoist(args);

        assert_eq!(todoist_args.project_id, None);
    }

    #[test]
    fn general_update_can_clear_due_date_while_updating_other_fields() {
        let args = UpdateTaskArgs {
            content: Some("Clarified task".to_string()),
            description: Some("Preserved context".to_string()),
            project_remote_id: Some("destination-project".to_string()),
            section_remote_id: None,
            parent_remote_id: None,
            priority: Some(1),
            due_date: None,
            due_datetime: None,
            clear_due_date: true,
            duration: None,
            labels: None,
        };

        let todoist_args = TodoistBackend::task_update_args_to_todoist(&args);

        assert_eq!(todoist_args.content.as_deref(), Some("Clarified task"));
        assert_eq!(todoist_args.description.as_deref(), Some("Preserved context"));
        assert_eq!(todoist_args.priority, Some(1));
        assert_eq!(todoist_args.due_string.as_deref(), Some("no date"));
    }

    #[test]
    fn completed_task_uses_todoists_completion_timestamp() {
        let api_task: crate::todoist::Task = serde_json::from_value(serde_json::json!({
            "id": "completed-task",
            "user_id": "user",
            "content": "Exercise",
            "description": "",
            "project_id": "inbox",
            "section_id": null,
            "parent_id": null,
            "added_by_uid": "user",
            "assigned_by_uid": null,
            "responsible_uid": null,
            "labels": [],
            "deadline": null,
            "duration": null,
            "checked": true,
            "is_deleted": false,
            "added_at": "2026-07-18T10:00:00Z",
            "completed_at": "2026-07-18T14:30:00Z",
            "completed_by_uid": "user",
            "updated_at": "2026-07-18T14:30:00Z",
            "due": {
                "date": "2026-07-18",
                "string": "today",
                "lang": "en",
                "is_recurring": false
            },
            "priority": 1,
            "child_order": 0,
            "note_count": 0,
            "day_order": 0,
            "is_collapsed": false
        }))
        .unwrap();

        let task = TodoistBackend::task_to_backend(&api_task);
        assert!(task.is_completed);
        assert_eq!(task.completed_at.as_deref(), Some("2026-07-18T14:30:00Z"));
    }

    #[test]
    fn completed_endpoint_uses_items_response_shape() {
        let response: CompletedTasksPage = serde_json::from_value(serde_json::json!({
            "items": [],
            "next_cursor": null
        }))
        .unwrap();

        assert!(response.items.is_empty());
        assert!(response.next_cursor.is_none());
    }

    #[test]
    fn sync_response_accepts_sparse_deletion_tombstones() {
        let response: SyncResponse = serde_json::from_value(serde_json::json!({
            "sync_token": "next-token",
            "full_sync": false,
            "projects": [{"id": "deleted-project", "is_deleted": true}],
            "items": [{"id": "deleted-task", "is_deleted": true}],
            "labels": [{"id": "deleted-label", "is_deleted": true}],
            "sections": [{"id": "deleted-section", "is_deleted": true}]
        }))
        .unwrap();

        assert_eq!(response.sync_token, "next-token");
        assert!(!response.full_sync);
        assert!(response.projects[0].is_deleted);
        assert!(response.items[0].is_deleted);
        assert!(response.labels[0].is_deleted);
        assert!(response.sections[0].is_deleted);
    }

    #[test]
    fn incremental_task_conversion_preserves_completion_and_recurrence() {
        let task: SyncTask = serde_json::from_value(serde_json::json!({
            "id": "changed-task",
            "content": "Changed",
            "description": "",
            "project_id": "inbox",
            "priority": 2,
            "child_order": 4,
            "checked": true,
            "completed_at": "2026-07-24T14:00:00Z",
            "labels": ["work"],
            "due": {
                "string": "every day",
                "date": "2026-07-24",
                "is_recurring": true
            }
        }))
        .unwrap();

        let task = TodoistBackend::sync_task_to_backend(task).unwrap();
        assert!(task.is_completed);
        assert!(task.is_recurring);
        assert_eq!(task.completed_at.as_deref(), Some("2026-07-24T14:00:00Z"));
        assert_eq!(task.labels, vec!["work"]);
    }
}
