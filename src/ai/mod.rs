//! AI proposal generation with a bounded, read-only tool-calling loop.

use crate::config::AiConfig;
use crate::entities::{project, task};
use crate::priority::TaskPriority;
use crate::ui::core::{AiProjectDestination, AiProposedAction, AiProposedActionKind, AiTaskProposal};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use uuid::Uuid;

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MAX_TOOL_ROUNDS: usize = 4;
const MAX_TOOL_RESULTS: usize = 50;

#[derive(Debug, Clone)]
pub struct AiTaskRequest {
    pub task: task::Model,
    pub context: String,
    pub project_name: String,
    pub section_name: Option<String>,
    /// Local cached data exposed only when the model requests a read-only tool.
    pub available_projects: Vec<project::Model>,
    pub available_tasks: Vec<task::Model>,
    pub previous_proposal: Option<AiTaskProposal>,
    pub revision_request: Option<String>,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn generate_proposal(&self, request: AiTaskRequest) -> Result<AiTaskProposal>;
}

#[derive(Clone)]
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    responses_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProviderProposal {
    summary: String,
    completion_note: Option<String>,
    projects_to_create: Vec<ProposedProject>,
    tasks_to_create: Vec<ProposedTask>,
    move_original_to: Option<ProviderDestination>,
    complete_original: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProposedProject {
    reference: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProposedTask {
    content: String,
    description: String,
    destination: ProviderDestination,
    priority: TaskPriority,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProviderDestination {
    kind: String,
    project_uuid: Option<Uuid>,
    project_reference: Option<String>,
    project_name: String,
}

#[derive(Debug)]
struct FunctionCall {
    call_id: String,
    name: String,
    arguments: String,
}

impl OpenAiProvider {
    pub fn from_config(config: &AiConfig) -> Result<Self> {
        let api_key = std::env::var(&config.api_key_env).with_context(|| {
            format!(
                "{} is not set. Export it before starting Terminalist.",
                config.api_key_env
            )
        })?;
        if api_key.trim().is_empty() {
            anyhow::bail!("{} is empty", config.api_key_env);
        }

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Failed to create OpenAI HTTP client")?,
            api_key,
            model: config.model.clone(),
            responses_url: RESPONSES_URL.to_string(),
        })
    }

    fn request_body(&self, input: &[Value]) -> Value {
        json!({
            "model": self.model,
            "store": false,
            "max_output_tokens": 1200,
            "reasoning": {"effort": "low"},
            "instructions": concat!(
                "Role: Help the user manage their Todoist tasks.\n",
                "Generate a proposal based on the context provided by the user.\n",
                "Propose only actions supported by the response schema.\n",
                "Provide a concise recommendation and populate the applicable action fields.\n",
                "When a revision is requested, return a complete replacement proposal that reflects it in both the recommendation and applicable action fields."
            ),
            "input": input,
            "tools": tool_definitions(),
            "tool_choice": "auto",
            "parallel_tool_calls": true,
            "text": {
                "verbosity": "low",
                "format": {
                    "type": "json_schema",
                    "name": "terminalist_task_proposal",
                    "strict": true,
                    "schema": proposal_schema()
                }
            }
        })
    }

    fn initial_input(request: &AiTaskRequest) -> Vec<Value> {
        let previous_proposal = request.previous_proposal.as_ref().map(|proposal| {
            json!({
                "summary": proposal.summary,
                "actions": proposal.actions.iter().map(|action| {
                    json!({
                        "enabled": action.enabled,
                        "description": action.description(),
                    })
                }).collect::<Vec<_>>()
            })
        });
        vec![json!({
            "role": "user",
            "content": serde_json::to_string(&json!({
                "selected_task": {
                    "uuid": request.task.uuid,
                    "title": request.task.content,
                    "description": request.task.description,
                    "project": request.project_name,
                    "section": request.section_name,
                    "priority": TaskPriority::from_todoist(request.task.priority),
                    "due": request.task.due_datetime.as_ref().or(request.task.due_date.as_ref()),
                },
                "user_context": request.context,
                "previous_proposal": previous_proposal,
                "revision_request": request.revision_request,
            })).expect("task context is serializable")
        })]
    }

    async fn send(&self, input: &[Value]) -> Result<Value> {
        let response = self
            .client
            .post(&self.responses_url)
            .bearer_auth(&self.api_key)
            .json(&self.request_body(input))
            .send()
            .await
            .context("OpenAI request failed")?;
        let status = response.status();
        let value: Value = response.json().await.context("OpenAI returned invalid JSON")?;
        if !status.is_success() {
            let message = value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("unknown OpenAI error");
            anyhow::bail!("OpenAI request failed ({status}): {message}");
        }
        Ok(value)
    }

    fn function_calls(value: &Value) -> Result<Vec<FunctionCall>> {
        value
            .get("output")
            .and_then(Value::as_array)
            .context("OpenAI response contained no output array")?
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
            .map(|item| {
                Ok(FunctionCall {
                    call_id: item
                        .get("call_id")
                        .and_then(Value::as_str)
                        .context("Function call omitted call_id")?
                        .to_string(),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .context("Function call omitted name")?
                        .to_string(),
                    arguments: item
                        .get("arguments")
                        .and_then(Value::as_str)
                        .context("Function call omitted arguments")?
                        .to_string(),
                })
            })
            .collect()
    }

    fn parse_response(value: &Value, request: &AiTaskRequest) -> Result<AiTaskProposal> {
        let text = value
            .get("output")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find_map(|item| {
                    item.get("content")?.as_array()?.iter().find_map(|content| {
                        (content.get("type")?.as_str()? == "output_text")
                            .then(|| content.get("text")?.as_str())
                            .flatten()
                    })
                })
            })
            .context("OpenAI response contained no structured proposal")?;
        let raw: ProviderProposal = serde_json::from_str(text).context("OpenAI returned an invalid proposal")?;
        raw.into_proposal(&request.available_projects)
    }

    fn execute_tool(call: &FunctionCall, request: &AiTaskRequest) -> Value {
        match call.name.as_str() {
            "list_projects" => {
                #[derive(Deserialize)]
                struct Args {
                    query: Option<String>,
                }
                let args: Args = match serde_json::from_str(&call.arguments) {
                    Ok(args) => args,
                    Err(error) => return json!({"error": format!("Invalid arguments: {error}")}),
                };
                let query = args.query.unwrap_or_default().to_lowercase();
                let projects = request
                    .available_projects
                    .iter()
                    .filter(|project| query.is_empty() || project.name.to_lowercase().contains(&query))
                    .take(MAX_TOOL_RESULTS)
                    .map(project_value)
                    .collect::<Vec<_>>();
                json!({"projects": projects, "truncated": projects.len() == MAX_TOOL_RESULTS})
            }
            "list_tasks" => {
                #[derive(Deserialize)]
                struct Args {
                    project_uuid: Option<Uuid>,
                    include_completed: bool,
                }
                let args: Args = match serde_json::from_str(&call.arguments) {
                    Ok(args) => args,
                    Err(error) => return json!({"error": format!("Invalid arguments: {error}")}),
                };
                if let Some(uuid) = args.project_uuid {
                    if !request.available_projects.iter().any(|project| project.uuid == uuid) {
                        return json!({"error": "Unknown project UUID"});
                    }
                }
                let tasks = request
                    .available_tasks
                    .iter()
                    .filter(|task| !task.is_deleted)
                    .filter(|task| args.include_completed || !task.is_completed)
                    .filter(|task| args.project_uuid.is_none_or(|uuid| task.project_uuid == uuid))
                    .take(MAX_TOOL_RESULTS)
                    .map(task_value)
                    .collect::<Vec<_>>();
                json!({"tasks": tasks, "truncated": tasks.len() == MAX_TOOL_RESULTS})
            }
            "search_tasks" => {
                #[derive(Deserialize)]
                struct Args {
                    query: String,
                    project_uuid: Option<Uuid>,
                    include_completed: bool,
                }
                let args: Args = match serde_json::from_str(&call.arguments) {
                    Ok(args) => args,
                    Err(error) => return json!({"error": format!("Invalid arguments: {error}")}),
                };
                if let Some(uuid) = args.project_uuid {
                    if !request.available_projects.iter().any(|project| project.uuid == uuid) {
                        return json!({"error": "Unknown project UUID"});
                    }
                }
                let query = args.query.to_lowercase();
                let tasks = request
                    .available_tasks
                    .iter()
                    .filter(|task| !task.is_deleted)
                    .filter(|task| args.include_completed || !task.is_completed)
                    .filter(|task| args.project_uuid.is_none_or(|uuid| task.project_uuid == uuid))
                    .filter(|task| {
                        task.content.to_lowercase().contains(&query)
                            || task
                                .description
                                .as_deref()
                                .is_some_and(|description| description.to_lowercase().contains(&query))
                    })
                    .take(MAX_TOOL_RESULTS)
                    .map(task_value)
                    .collect::<Vec<_>>();
                json!({"tasks": tasks, "truncated": tasks.len() == MAX_TOOL_RESULTS})
            }
            _ => json!({"error": "Unknown read-only tool"}),
        }
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn generate_proposal(&self, request: AiTaskRequest) -> Result<AiTaskProposal> {
        let mut input = Self::initial_input(&request);
        let mut seen_calls = HashSet::new();

        for _ in 0..=MAX_TOOL_ROUNDS {
            let response = self.send(&input).await?;
            let calls = Self::function_calls(&response)?;
            if calls.is_empty() {
                return Self::parse_response(&response, &request);
            }

            let output = response
                .get("output")
                .and_then(Value::as_array)
                .context("OpenAI response contained no output array")?;
            input.extend(output.iter().cloned());
            for call in calls {
                let signature = format!("{}:{}", call.name, call.arguments);
                let result = if seen_calls.insert(signature) {
                    Self::execute_tool(&call, &request)
                } else {
                    json!({"error": "Repeated identical tool call"})
                };
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call.call_id,
                    "output": serde_json::to_string(&result).expect("tool result is serializable")
                }));
            }
        }
        anyhow::bail!("OpenAI exceeded the maximum number of read-only tool rounds")
    }
}

impl ProviderProposal {
    fn into_proposal(self, available_projects: &[project::Model]) -> Result<AiTaskProposal> {
        if self.summary.trim().is_empty() {
            anyhow::bail!("AI proposal summary was empty");
        }
        let existing_projects = available_projects
            .iter()
            .map(|project| (project.uuid, project.name.as_str()))
            .collect::<HashMap<_, _>>();
        let proposed_projects = self
            .projects_to_create
            .iter()
            .map(|project| (project.reference.clone(), project.name.clone()))
            .collect::<HashMap<_, _>>();
        if proposed_projects.len() != self.projects_to_create.len() {
            anyhow::bail!("AI proposal used duplicate project references");
        }

        let mut actions = Vec::new();
        for project in self.projects_to_create {
            if project.reference.trim().is_empty() || project.name.trim().is_empty() {
                anyhow::bail!("AI project creation requires a reference and name");
            }
            actions.push(AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::CreateProject {
                    reference: project.reference,
                    name: project.name,
                },
            });
        }
        if let Some(content) = self.completion_note.filter(|value| !value.trim().is_empty()) {
            actions.push(AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::AddCompletionNote { content },
            });
        }
        for task in self.tasks_to_create {
            actions.push(AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::CreateTask {
                    content: task.content,
                    description: task.description,
                    destination: task.destination.into_destination(&existing_projects, &proposed_projects)?,
                    priority: task.priority.todoist_value(),
                },
            });
        }
        if let Some(destination) = self.move_original_to {
            actions.push(AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::MoveOriginalTask {
                    destination: destination.into_destination(&existing_projects, &proposed_projects)?,
                },
            });
        }
        if self.complete_original {
            actions.push(AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::CompleteTask,
            });
        }
        let proposal = AiTaskProposal {
            summary: self.summary,
            actions,
        };
        proposal.validate().map_err(anyhow::Error::msg)?;
        Ok(proposal)
    }
}

impl ProviderDestination {
    fn into_destination(
        self,
        existing: &HashMap<Uuid, &str>,
        proposed: &HashMap<String, String>,
    ) -> Result<AiProjectDestination> {
        match self.kind.as_str() {
            "existing" => {
                let uuid = self.project_uuid.context("Existing destination omitted project_uuid")?;
                let actual_name = existing.get(&uuid).context("AI selected an unknown project UUID")?;
                Ok(AiProjectDestination::Existing {
                    uuid,
                    name: (*actual_name).to_string(),
                })
            }
            "proposed" => {
                let reference = self
                    .project_reference
                    .context("Proposed destination omitted project_reference")?;
                let actual_name = proposed
                    .get(&reference)
                    .context("AI selected an unknown proposed-project reference")?;
                Ok(AiProjectDestination::Proposed {
                    reference,
                    name: actual_name.clone(),
                })
            }
            _ => anyhow::bail!("AI returned an unknown destination kind"),
        }
    }
}

fn project_value(project: &project::Model) -> Value {
    json!({
        "uuid": project.uuid,
        "name": project.name,
        "parent_uuid": project.parent_uuid,
        "is_inbox": project.is_inbox_project,
    })
}

fn task_value(task: &task::Model) -> Value {
    json!({
        "uuid": task.uuid,
        "title": task.content,
        "description": task.description,
        "project_uuid": task.project_uuid,
        "priority": TaskPriority::from_todoist(task.priority),
        "due": task.due_datetime.as_ref().or(task.due_date.as_ref()),
        "completed": task.is_completed,
    })
}

fn tool_definitions() -> Value {
    json!([
        {
            "type": "function",
            "name": "list_projects",
            "description": "List existing Todoist projects, optionally filtered by a case-insensitive name query.",
            "strict": true,
            "parameters": {
                "type": "object",
                "additionalProperties": false,
                "properties": {"query": {"type": ["string", "null"]}},
                "required": ["query"]
            }
        },
        {
            "type": "function",
            "name": "list_tasks",
            "description": "List cached Todoist tasks. Supply project_uuid for one project, or null for tasks across all projects.",
            "strict": true,
            "parameters": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "project_uuid": {"type": ["string", "null"]},
                    "include_completed": {"type": "boolean"}
                },
                "required": ["project_uuid", "include_completed"]
            }
        },
        {
            "type": "function",
            "name": "search_tasks",
            "description": "Search cached task titles and descriptions, optionally within one project.",
            "strict": true,
            "parameters": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {"type": "string"},
                    "project_uuid": {"type": ["string", "null"]},
                    "include_completed": {"type": "boolean"}
                },
                "required": ["query", "project_uuid", "include_completed"]
            }
        }
    ])
}

fn destination_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "kind": {"type": "string", "enum": ["existing", "proposed"]},
            "project_uuid": {"type": ["string", "null"]},
            "project_reference": {"type": ["string", "null"]},
            "project_name": {"type": "string"}
        },
        "required": ["kind", "project_uuid", "project_reference", "project_name"]
    })
}

fn proposal_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "summary": {"type": "string", "minLength": 1},
            "completion_note": {"type": ["string", "null"]},
            "projects_to_create": {
                "type": "array",
                "maxItems": 2,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "reference": {"type": "string", "minLength": 1},
                        "name": {"type": "string", "minLength": 1}
                    },
                    "required": ["reference", "name"]
                }
            },
            "tasks_to_create": {
                "type": "array",
                "maxItems": 2,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "content": {"type": "string", "minLength": 1},
                        "description": {"type": "string"},
                        "destination": destination_schema(),
                        "priority": {
                            "type": "string",
                            "enum": ["low", "medium", "high", "urgent"]
                        }
                    },
                    "required": ["content", "description", "destination", "priority"]
                }
            },
            "move_original_to": {
                "anyOf": [destination_schema(), {"type": "null"}]
            },
            "complete_original": {"type": "boolean"}
        },
        "required": [
            "summary",
            "completion_note",
            "projects_to_create",
            "tasks_to_create",
            "move_original_to",
            "complete_original"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn project(name: &str) -> project::Model {
        project::Model {
            uuid: Uuid::new_v4(),
            backend_uuid: Uuid::new_v4(),
            remote_id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            is_favorite: false,
            is_inbox_project: false,
            order_index: 0,
            parent_uuid: None,
        }
    }

    fn task(project_uuid: Uuid) -> task::Model {
        task::Model {
            uuid: Uuid::new_v4(),
            backend_uuid: Uuid::new_v4(),
            remote_id: Uuid::new_v4().to_string(),
            content: "Task".into(),
            description: None,
            project_uuid,
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
        }
    }

    #[test]
    fn maps_existing_and_proposed_destinations() {
        let existing = project("Family");
        let proposal = ProviderProposal {
            summary: "Preserve the result and defer optional work.".into(),
            completion_note: Some("Explained the current workaround.".into()),
            projects_to_create: vec![ProposedProject {
                reference: "future".into(),
                name: "Future experiments".into(),
            }],
            tasks_to_create: vec![
                ProposedTask {
                    content: "Follow up".into(),
                    description: "Preserve this follow-up.".into(),
                    destination: ProviderDestination {
                        kind: "existing".into(),
                        project_uuid: Some(existing.uuid),
                        project_reference: None,
                        project_name: "ignored".into(),
                    },
                    priority: TaskPriority::Urgent,
                },
                ProposedTask {
                    content: "Experiment".into(),
                    description: "Try this later.".into(),
                    destination: ProviderDestination {
                        kind: "proposed".into(),
                        project_uuid: None,
                        project_reference: Some("future".into()),
                        project_name: "Future experiments".into(),
                    },
                    priority: TaskPriority::Urgent,
                },
            ],
            move_original_to: None,
            complete_original: true,
        }
        .into_proposal(std::slice::from_ref(&existing))
        .unwrap();

        assert!(matches!(
            proposal.actions[2].kind,
            AiProposedActionKind::CreateTask {
                destination: AiProjectDestination::Existing { uuid, .. },
                ..
            } if uuid == existing.uuid
        ));
        assert!(matches!(
            proposal.actions[3].kind,
            AiProposedActionKind::CreateTask {
                destination: AiProjectDestination::Proposed { ref reference, .. },
                priority: 4,
                ..
            } if reference == "future"
        ));
    }

    #[test]
    fn ai_schema_uses_named_priorities() {
        let schema = proposal_schema();
        assert_eq!(
            schema["properties"]["tasks_to_create"]["items"]["properties"]["priority"]["enum"],
            json!(["low", "medium", "high", "urgent"])
        );
    }

    #[test]
    fn list_tasks_can_scope_to_project_or_all_projects() {
        let first = project("First");
        let second = project("Second");
        let request = AiTaskRequest {
            task: task(first.uuid),
            context: String::new(),
            project_name: first.name.clone(),
            section_name: None,
            available_projects: vec![first.clone(), second.clone()],
            available_tasks: vec![task(first.uuid), task(second.uuid)],
            previous_proposal: None,
            revision_request: None,
        };
        let scoped = OpenAiProvider::execute_tool(
            &FunctionCall {
                call_id: "1".into(),
                name: "list_tasks".into(),
                arguments: json!({"project_uuid": first.uuid, "include_completed": false}).to_string(),
            },
            &request,
        );
        let all = OpenAiProvider::execute_tool(
            &FunctionCall {
                call_id: "2".into(),
                name: "list_tasks".into(),
                arguments: json!({"project_uuid": null, "include_completed": false}).to_string(),
            },
            &request,
        );
        assert_eq!(scoped["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(all["tasks"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn revision_input_includes_previous_proposal_and_requested_changes() {
        let family = project("Family");
        let previous = AiTaskProposal::mock_for(&task(family.uuid), "Handled", Some(family.uuid));
        let input = OpenAiProvider::initial_input(&AiTaskRequest {
            task: task(family.uuid),
            context: "Original context".into(),
            project_name: family.name.clone(),
            section_name: None,
            available_projects: vec![family],
            available_tasks: vec![],
            previous_proposal: Some(previous),
            revision_request: Some("Use the existing project".into()),
        });
        let content = input[0]["content"].as_str().unwrap();
        assert!(content.contains("previous_proposal"));
        assert!(content.contains("Use the existing project"));
        assert!(content.contains("Original context"));
    }

    #[tokio::test]
    #[ignore = "requires loopback sockets"]
    async fn provider_executes_tool_call_before_parsing_proposal() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for response_body in [
                json!({
                    "status": "completed",
                    "output": [{
                        "type": "function_call",
                        "id": "fc_1",
                        "call_id": "call_1",
                        "name": "list_projects",
                        "arguments": "{\"query\":null}"
                    }]
                })
                .to_string(),
                json!({
                    "status": "completed",
                    "output": [{
                        "type": "message",
                        "content": [{
                            "type": "output_text",
                            "text": serde_json::to_string(&ProviderProposal {
                                summary: "Use the existing project.".into(),
                                completion_note: None,
                                projects_to_create: vec![],
                                tasks_to_create: vec![],
                                move_original_to: None,
                                complete_original: true,
                            }).unwrap()
                        }]
                    }]
                })
                .to_string(),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request_bytes = Vec::new();
                let mut chunk = [0_u8; 4096];
                loop {
                    let count = socket.read(&mut chunk).await.unwrap();
                    if count == 0 {
                        break;
                    }
                    request_bytes.extend_from_slice(&chunk[..count]);
                    if let Some(header_end) = request_bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request_bytes[..header_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length: ")?.parse().ok())
                            .unwrap_or(0);
                        if request_bytes.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                }
                let request_text = String::from_utf8_lossy(&request_bytes);
                if response_body.contains("output_text") {
                    assert!(request_text.contains("function_call_output"));
                    assert!(request_text.contains("Family"));
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let family = project("Family");
        let provider = OpenAiProvider {
            client: reqwest::Client::new(),
            api_key: "test".into(),
            model: "test".into(),
            responses_url: format!("http://{address}"),
        };
        let proposal = provider
            .generate_proposal(AiTaskRequest {
                task: task(family.uuid),
                context: "Find the best project.".into(),
                project_name: family.name.clone(),
                section_name: None,
                available_projects: vec![family],
                available_tasks: vec![],
                previous_proposal: None,
                revision_request: None,
            })
            .await
            .unwrap();
        server.await.unwrap();
        assert!(matches!(
            proposal.actions.last().map(|action| &action.kind),
            Some(AiProposedActionKind::CompleteTask)
        ));
    }

    #[tokio::test]
    #[ignore = "makes a live OpenAI API request"]
    async fn live_provider_accepts_tools_and_structured_proposal() {
        let family = project("Family");
        let provider = OpenAiProvider::from_config(&AiConfig::default()).unwrap();
        let proposal = provider
            .generate_proposal(AiTaskRequest {
                task: task(family.uuid),
                context: "Use list_projects before proposing that I complete this test task. Do not create anything."
                    .into(),
                project_name: family.name.clone(),
                section_name: None,
                available_projects: vec![family],
                available_tasks: vec![],
                previous_proposal: None,
                revision_request: None,
            })
            .await
            .unwrap();
        assert!(!proposal.summary.is_empty());
    }
}
