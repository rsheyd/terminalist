# Terminalist workspace

- Preserve the Todoist/local-cache distinction: remote deletion happens before local tombstoning.
- Run `cargo fmt --all -- --check` and `cargo test` after Rust changes.

## File map

- `src/ai/`: Narrow AI provider boundary, structured proposal generation, and validation.
- `src/backend/`: Backend API adapters and full/incremental sync payloads.
- `src/priority.rs`: Human-readable task priorities and Todoist API conversion.
- `src/sync/`: Todoist synchronization and local cache reconciliation.
- `src/repositories/`: SQLite queries.
- `src/ui/`: actions, navigation, dialogs, and TUI components.
- `tests/`: integration and UI behavior tests.
- `docs/KEYBOARD_SHORTCUTS.md`: user-facing controls and view behavior.
- `docs/POST_PR188_CHANGES.md`: branch-local feature ledger and future PR source notes.
- `docs/AI_TASK_MANAGEMENT_PLAN.md`: proposed AI-assisted task-management workflow and implementation plan.
