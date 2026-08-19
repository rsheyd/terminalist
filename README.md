# Terminalist Edge

[![CI](https://github.com/rsheyd/terminalist-edge/actions/workflows/ci.yml/badge.svg)](https://github.com/rsheyd/terminalist-edge/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/rsheyd/terminalist-edge)](https://github.com/rsheyd/terminalist-edge/releases)
[![Rust 1.92+](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Terminalist Edge is an independently maintained downstream edition of [Terminalist](https://github.com/romaintb/terminalist), a keyboard-first Todoist client for the terminal. Edge develops expanded task-management workflows—including smart views, recoverable local Trash, incremental synchronization, task editing, and review-first AI proposals—while remaining open to contributing suitable changes upstream.

<img src="docs/images/screenshot1.png" width="48%" alt="Terminalist Edge task list and smart views"> <img src="docs/images/screenshot2.png" width="48%" alt="Terminalist Edge task details">

## Status and distribution

Terminalist Edge is experimental software that can create, edit, complete, and delete real Todoist data. Review the limitations and start with data you can recover.

The latest GitHub release is `v0.7.2`. It is source-only and has no prebuilt assets. The `main` branch currently identifies itself as `0.7.3` and contains changes made after that release.

Homebrew, AUR, crates.io, and other packages named `terminalist` currently install the upstream project, not Terminalist Edge. This repository also retains the upstream `terminalist` crate, binary, configuration, and local-data names for compatibility, so installing Edge can replace an upstream `terminalist` command and both editions can address the same local files. Back up the Terminalist data directory before switching editions and do not alternate between them unless their schema compatibility has been verified.

## What Edge adds

- Today, Agenda, Tomorrow, Upcoming, and Trash smart views in an always-visible navigation bar
- Incremental Todoist synchronization with transactional sync-token updates and full-sync recovery
- Task details plus title, description, schedule, project, priority, completion, and due-date workflows
- Context-aware task creation and responsive navigation while background refreshes finish
- A collapsible Projects & Labels rail with persisted layout state
- Locally retained Trash records for 30 days after remote deletion, with recreation-based restore
- Review-first OpenAI proposals with bounded, read-only project/task lookup tools and explicit confirmation before proposed Todoist mutations

Inherited Terminalist capabilities include local SQLite caching, project and label browsing, task search, keyboard and mouse navigation, configurable display settings, and periodic or manual Todoist synchronization.

## Requirements

- Rust 1.92 or later; the repository pins the expected toolchain in `rust-toolchain.toml`
- A Todoist account and personal API token
- A terminal with standard TUI and color support
- An OpenAI API key only if you choose to use AI task management

## Install from source

Install the latest published Edge release:

```bash
git clone https://github.com/rsheyd/terminalist-edge.git
cd terminalist-edge
git checkout v0.7.2
cargo install --locked --path .
```

To test current development instead, stay on `main` before running `cargo install`. Both paths install a binary named `terminalist`.

Set your Todoist token in the environment, then start the app:

```bash
export TODOIST_API_TOKEN="your-token"
terminalist
```

The token is read from the environment and then stored in Terminalist's local SQLite database as part of its Todoist backend configuration. That database is not encrypted by Terminalist; protect it with normal operating-system account and disk security. See [Privacy and data handling](PRIVACY.md) before using real account data.

## First run

1. Create a Todoist API token from [Todoist integration settings](https://todoist.com/prefs/integrations).
2. Export `TODOIST_API_TOKEN` in the shell that launches Terminalist Edge.
3. Run `terminalist`; the first startup creates the local database and synchronizes Todoist data.
4. Use `Left` and `Right` for smart views, `[` and `]` for projects and labels, `j` and `k` for tasks, `Enter` for details, and `?` for built-in help.
5. Press `r` whenever you want to request a manual synchronization.

Generate an optional configuration file with:

```bash
terminalist --generate-config
```

See the [configuration guide](docs/CONFIGURATION.md) for platform-specific paths and available settings.

## Important behavior and limitations

- Most ordinary task actions are sent to Todoist immediately after their confirmation dialog or command; this is not an offline editor with a later commit step.
- Deleting a task first deletes it remotely, then retains a local tombstone for up to 30 days. Restoring from Trash creates a new Todoist task from cached fields; it does not recover the original remote task identity, comments, or every server-side relationship.
- Emptying Trash removes only the retained local tombstones because the corresponding tasks were already deleted remotely.
- Todoist does not provide an atomic transaction for the multi-action AI workflow. Approved actions run in a safe order, stop on failure, report partial results, and complete the original task last.
- The AI feature is optional, but when invoked it sends task information and user-supplied context to OpenAI and can return bounded portions of cached project or task data through model-requested read-only tools.
- Edge does not currently have a distinct package, executable, configuration, or database namespace from upstream Terminalist.

## AI task management

From a task's details, press `m` to open **Manage task with AI…**. Terminalist sends the selected task and your explanation to the configured OpenAI model, validates the structured proposal, and shows the recommendation and individual proposed actions. You can revise the proposal, disable actions, cancel without changes, or continue to a separate application confirmation.

The feature is not a general autonomous agent: its proposal schema and mutation types are restricted, and model-requested context tools can only read bounded cached project/task data. Applying approved actions can nevertheless change real Todoist data.

Configuration uses an environment-variable name rather than storing the OpenAI key itself:

```toml
[ai]
model = "gpt-5.6-sol"
api_key_env = "OPENAI_API_KEY"
```

```bash
export OPENAI_API_KEY="your-key"
```

The checked-in default environment-variable name reflects a personal development setup, so public users should set `ai.api_key_env` explicitly. See [AI task management and data flow](docs/AI_TASK_MANAGEMENT.md) for the exact review boundary and information shared with OpenAI.

## Essential controls

| Key | Action |
|-----|--------|
| `j` / `k` | Move between tasks |
| `Left` / `Right` | Cycle smart views |
| `]` / `[` or `J` / `K` | Move through projects and labels |
| `Enter` | Open task details |
| `a` | Create a task |
| `Space` | Complete or reopen the current or marked tasks |
| `x` | Mark a task for bulk actions |
| `/` | Search cached tasks |
| `r` | Synchronize with Todoist |
| `b` | Collapse or expand Projects & Labels |
| `?` | Open help |
| `q` | Quit |

See [keyboard shortcuts](docs/KEYBOARD_SHORTCUTS.md) for editing, scheduling, Trash, AI review, mouse, and dialog controls.

## Documentation

- [Configuration](docs/CONFIGURATION.md)
- [Keyboard shortcuts](docs/KEYBOARD_SHORTCUTS.md)
- [AI task management and data flow](docs/AI_TASK_MANAGEMENT.md)
- [Privacy and local data](PRIVACY.md)
- [Development](docs/DEVELOPMENT.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Changelog](CHANGELOG.md)
- [Releasing](docs/RELEASING.md)

## Relationship to upstream

Terminalist Edge is maintained by [Roman Sheydvasser](https://github.com/rsheyd) and is not an official upstream release. The original Terminalist project is maintained at [romaintb/terminalist](https://github.com/romaintb/terminalist). Edge retains upstream package metadata and copyright notices while its downstream changes are evaluated and, where practical, proposed upstream as focused contributions.

If you want upstream Terminalist rather than the Edge feature set, use the upstream repository and its official package instructions.

## License

Terminalist Edge is distributed under the [MIT License](LICENSE). The included copyright notice identifies the upstream copyright holder and must be preserved with copies or substantial portions of the software.
