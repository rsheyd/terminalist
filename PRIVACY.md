# Privacy and local data

Terminalist Edge connects directly to Todoist and, only when the optional AI workflow is invoked, to OpenAI. It does not include a project-operated account service, analytics, advertising, or crash-reporting integration in the current source.

## Local storage

Terminalist Edge stores a persistent SQLite cache named `terminalist.db` beneath the operating system's standard application-data directory in a `terminalist` subdirectory. Typical locations are:

- macOS: `~/Library/Application Support/terminalist/terminalist.db`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/terminalist/terminalist.db`
- Windows: the user-local application-data directory under `terminalist\terminalist.db`

The database contains cached Todoist projects, sections, labels, tasks, completion and deletion state, synchronization metadata, and backend configuration. After reading `TODOIST_API_TOKEN` from the environment, the current implementation also stores that token as JSON-encoded backend credentials in this unencrypted SQLite database. Protect the file with operating-system account and disk security, do not include it in bug reports, and remove credentials before sharing a copy.

Configuration and UI state use the operating system's standard configuration directory under `terminalist`. A `terminalist.toml` file in the current working directory takes precedence over the user configuration. Optional file logging writes `terminalist.log` beside the user configuration; review logs before sharing them.

Edge currently shares these names and paths with upstream Terminalist. Back up the directory before switching editions or testing a build with a different database schema.

## Todoist

Normal synchronization sends the Todoist API token and task-management requests directly to Todoist over HTTPS. The local cache contains Todoist data so the interface can remain responsive and support cached startup behavior.

Creates, edits, completions, project changes, comments, and deletions performed in the interface can modify the Todoist account. Remote task deletion occurs before Terminalist retains its local Trash record. Restoring that record recreates a task through Todoist and does not reverse the original deletion.

Todoist handles these requests under its own terms and privacy practices.

## Optional OpenAI workflow

Terminalist Edge contacts OpenAI only when you request or revise an AI task-management proposal. It sends:

- the selected task's local UUID, title, description, project, section, priority, and due value;
- the context or revision request you type;
- a summary of the previous proposal when requesting a revision; and
- bounded cached project or task records when the model requests the available read-only tools.

Project tool results can include local UUIDs, names, parent relationships, and Inbox status. Task tool results can include local UUIDs, titles, descriptions, project UUIDs, priorities, due values, and completion status. Deleted tasks are excluded. A tool response is limited to 50 results, but the model can request more than one tool call during a proposal.

Requests use OpenAI's Responses API with response storage disabled in the request. The OpenAI API key is read from the environment variable named by `ai.api_key_env`; Terminalist does not write the key value to its configuration or database. OpenAI still processes submitted data under the API terms and privacy commitments applicable to your account.

Proposal generation is read-only with respect to Todoist. Terminalist parses the response into a restricted action schema and requires review plus a separate confirmation before applying enabled actions. Once confirmed, those actions can modify real Todoist data and may partially succeed because Todoist does not provide an atomic multi-action transaction.

## Removing local data

Quit every Terminalist or Terminalist Edge process before deleting local files. Remove the `terminalist` application-data directory to delete the SQLite cache and stored Todoist credential, and remove the `terminalist` configuration directory to delete configuration, UI state, and optional logs. This does not delete data already stored by Todoist or OpenAI.

Because upstream and Edge currently share local paths, deleting these directories also affects upstream Terminalist on the same operating-system account.

## Keeping this document current

Review this document whenever storage, credential handling, providers, analytics, crash reporting, synchronization, or package identity changes. Suspected discrepancies can be reported through [GitHub issues](https://github.com/rsheyd/terminalist-edge/issues).
