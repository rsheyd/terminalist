# AI task management

Terminalist Edge's optional AI workflow proposes coordinated task changes for review. It does not run continuously, execute arbitrary commands, or apply a model response automatically.

## Setup

Choose an OpenAI model and the name of an environment variable containing your API key:

```toml
[ai]
model = "gpt-5.6-sol"
api_key_env = "OPENAI_API_KEY"
```

Export that variable in the shell that launches Terminalist Edge:

```bash
export OPENAI_API_KEY="your-key"
```

The checked-in default uses `OPENAI_VALERIA_API_KEY`, which reflects a personal development environment. Set `api_key_env` explicitly for a public or shared setup. The key value remains in the environment and is not written to Terminalist configuration or its database.

## Workflow

1. Open a task with `Enter`.
2. Press `m` for **Manage task with AI…**.
3. Explain what changed or what decision you want help with.
4. Review the generated recommendation and proposed actions.
5. Use `Left` and `Right` to switch pages, move through actions with `j` and `k`, and press `Space` to enable or disable an action.
6. Press `r` if you want a complete revised proposal.
7. Continue to the separate application confirmation or cancel without changing Todoist.

Generating and revising proposals are read-only with respect to Todoist. Applying a confirmed proposal is not read-only.

## Allowed proposals

The structured proposal can preserve context on the original task, create projects, create related tasks, move the original task, and complete the original task. Terminalist validates identifiers and proposed-project references before presenting actions.

The provider cannot return arbitrary code or an unrestricted Todoist request. Only supported, validated action types can reach the review screen.

## Information sent to OpenAI

The initial request includes the selected task's title, description, project, section, priority, due value, local UUID, and the context you enter. A revision also includes the previous proposal's recommendation and action descriptions plus your requested changes.

The model can request three bounded read-only tools:

- `list_projects` returns matching cached project identifiers, names, parent relationships, and Inbox status.
- `list_tasks` returns up to 50 cached task identifiers, titles, descriptions, project identifiers, priorities, due values, and completion states, optionally scoped to one project.
- `search_tasks` returns up to 50 cached tasks whose title or description matches a query, optionally scoped to one project.

Deleted tasks are never returned by these tools. Completed tasks can be included when the model requests them. More than one tool request can occur during a proposal, so the total disclosed context can exceed 50 records.

Requests go to `https://api.openai.com/v1/responses` and set `store: false`. Consult [PRIVACY.md](../PRIVACY.md) for the broader local-data and provider boundary.

## Application safety boundary

Terminalist executes only enabled actions after explicit confirmation. It creates approved projects first, preserves context, creates or moves tasks, and completes the original task last. If an action fails, execution stops and the result screen distinguishes completed from pending work.

Todoist does not make the action sequence atomic. Review the proposed order and assume that a network or API failure can leave a partially applied result. Check Todoist before retrying.

## Current limitations

- OpenAI is the only implemented AI provider.
- The feature requires a separately funded or quota-enabled OpenAI API key.
- Model output can be wrong even when it matches the required schema; review content and destinations rather than relying only on validation.
- Disabling response storage in the request does not mean no third-party processing occurs.
- The feature is experimental and should not be treated as an autonomous productivity policy.
