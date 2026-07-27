# AI Task Management Plan

Status: proposed

## Purpose

Add a narrow, approval-based workflow that helps a user reconsider an existing
task, proposes coordinated Todoist changes, and applies only the actions the
user approves.

The initial use case is work that has partly or fully achieved its original
purpose but may leave behind an optional future project. For example, the
assistant might propose:

- adding a completion note to the selected task;
- creating an undated P4 successor in a `Someday` project;
- preserving research links and conditions for reconsidering the work; and
- completing the original task.

This feature should not behave like a general chatbot or silently reorganize a
Todoist account.

## Names

- Menu/action label: `Manage task with AI…`
- Dialog title: `AI Task Management`
- Internal action: `AiAssist`

## User Flow

1. Open a task's details.
2. Invoke `Manage task with AI…`, initially with the `m` key within the task
   details dialog.
3. Enter a short explanation of what changed and what decision needs help.
4. Generate a proposal.
5. Review and edit the proposed actions.
6. Enable or disable individual actions.
7. Apply the approved proposal or cancel without changing Todoist.
8. See which actions succeeded or where execution stopped.

The dialog should move through explicit states:

```text
Context entry → Generating → Proposal review → Applying → Result
```

## Proposal Model

AI output must be parsed into a typed, validated proposal rather than treated
as executable prose.

```rust
struct AiTaskProposal {
    summary: String,
    actions: Vec<ProposedTaskAction>,
}

enum ProposedTaskAction {
    AddComment {
        task_uuid: Uuid,
        content: String,
    },
    UpdateTask {
        task_uuid: Uuid,
        content: Option<String>,
        description: Option<String>,
        project_uuid: Option<Uuid>,
        priority: Option<i32>,
        clear_due_date: bool,
    },
    CreateTask {
        content: String,
        description: Option<String>,
        project_uuid: Uuid,
        priority: i32,
    },
    CompleteTask {
        task_uuid: Uuid,
    },
}
```

The exact types may change during implementation, but the boundary should
remain structured and restrictive.

## MVP Scope

Allow proposals to:

- add a comment to the selected task;
- edit its title or description;
- change its project or priority;
- remove its due date;
- create a small number of related tasks; and
- complete the selected task.

Exclude initially:

- hard deletion;
- reminders, deadlines, and recurrence changes;
- bulk changes to unrelated existing tasks;
- automatic execution without a proposal preview;
- arbitrary local-file access from links in task descriptions; and
- a general conversational assistant embedded in Terminalist.

## Required Application Work

### Richer Todoist Mutations

The backend argument types already represent descriptions, projects,
priorities, and due-date changes, but the current UI and `SyncService` expose
only narrower operations.

Add:

- a general task-update operation;
- richer task creation with description, project, and priority;
- Todoist comment creation;
- destination-project resolution, including `Someday`; and
- a composite proposal executor.

Comments are preferred for completion history. If comment support is deferred,
the MVP may append a clearly delimited note to the task description, but that
should be presented as a temporary limitation.

### Dialog and Actions

Add an AI task-management dialog reachable from `TaskDetails`. The dialog
should support multiline context entry, progress states, proposal review,
editing, per-action selection, confirmation, and partial-failure reporting.

Add `Action::AiAssist` as the internal entry point. Separate proposal
generation from proposal execution so that generating a proposal is read-only
and applying it is classified as a mutation.

### AI Provider Boundary

Put model-specific code behind a small interface, for example:

```rust
trait AiProvider {
    async fn propose_task_actions(
        &self,
        request: AiTaskRequest,
    ) -> Result<AiTaskProposal>;
}
```

The request should include:

- the selected task and its current metadata;
- its project and section;
- available destination projects;
- the user's explanation;
- the relevant documented productivity rules; and
- the allowed output schema.

Keep secrets out of `terminalist.toml`. Read credentials from an environment
variable or a system credential store. Non-secret settings such as provider
and model may live in Terminalist configuration.

## Safety and Validation

Before displaying or applying a proposal:

- reject unknown action types and invalid fields;
- reject nonexistent task or project identifiers;
- restrict updates to the selected task and tasks created by the proposal;
- cap the number of created tasks;
- require explicit approval for every mutation;
- do not invent a due date unless the user asked for scheduling; and
- never interpret explanatory prose as an application command.

Todoist does not make this multi-action workflow atomic. Execute approved
actions in the following order:

1. Preserve context on the original task.
2. Create successor tasks.
3. Apply project, priority, description, and scheduling changes.
4. Complete the original task last.

Stop on failure and report completed and pending actions. Completing the
original task last ensures that a partial failure does not make unfinished
work disappear.

## Testing

Add coverage for:

- proposal serialization, parsing, and validation;
- rejection of disallowed or unrelated actions;
- dialog states and keyboard behavior;
- editing and deselecting proposed actions;
- rich task creation and comment mutations;
- deterministic execution ordering;
- partial Todoist failures;
- completion occurring last;
- configuration defaults and credential errors; and
- mocked AI responses without calls to a live model.

After Rust changes, run:

```text
cargo fmt --all -- --check
cargo test
```

## Delivery Sequence

### Increment 1: Interaction Prototype

- Add the dialog and state transitions.
- Generate a hard-coded or mocked proposal.
- Validate the context-entry and proposal-review experience.
- Make no live AI request and apply no Todoist changes.

### Increment 2: Proposal Execution

- Add richer Todoist task and comment mutations.
- Implement typed proposal validation and ordered execution.
- Apply user-approved mock proposals.
- Test partial failures and local-cache reconciliation.

### Increment 3: AI Integration

- Add the provider interface and initial provider implementation.
- Add non-secret configuration and secure credential discovery.
- Build the constrained prompt and structured-output parser.
- Connect live proposal generation to the existing review and execution flow.

This ordering validates whether the interaction is useful before adding model
credentials, provider behavior, and prompt maintenance.

## Product Evaluation

Treat the feature as an experiment. After several real uses, evaluate:

- whether proposals save meaningful effort;
- whether users commonly edit or reject particular action types;
- whether `Someday` decisions stay understandable in Todoist;
- whether the feature preserves enough context;
- whether model latency and cost feel proportionate; and
- whether repeated patterns should become deterministic, non-AI commands.

Workflow semantics—what it means to complete, defer, move to `Someday`, or
create a successor task—belong in the personal productivity-system
documentation. Terminalist should implement those decisions as explicit,
visible, reversible Todoist mutations.
