# Post-PR #188 Branch Changes

Last updated: July 31, 2026

## Purpose

This file tracks user-visible features developed on `codex/post-pr188`, which is based on
the source branch for PR #188. Keep it current as work is added so it can serve as the
scope, testing summary, and starting point for a future pull request.

Do not include PR #188's existing consolidated work here. Record only changes introduced
after this branch diverged from `codex/test-full-stack`.

## Rough upstream PR roadmap

This is a provisional decomposition of the fork's integrated work into reviewable upstream
PRs. Before preparing each branch, compare it with the latest `upstream/main`, remove anything
already implemented there, and adjust boundaries when that produces a smaller or more coherent
change. Prefer independent PRs; stack a PR only when its code genuinely requires an earlier
unmerged change.

1. **Git revision in version output** — identify locally installed development builds and
   refresh the metadata when Git or source state changes. Draft PR
   [#201](https://github.com/romaintb/terminalist/pull/201).
2. **Persistent cache lifecycle** — preserve usable cached data across startup and sync
   failures, migrate existing databases, and replace remote snapshots transactionally.
3. **Typed background operations and stable IDs** — replace delimiter-encoded commands with
   typed operations and use UUIDs for project and label selections.
4. **Versioned view snapshots** — reject stale background results, retain the last accepted
   view on failure, and cover rapid-navigation races. Likely depends on item 3.
5. **Navigation and bulk task workflows** — improve pane navigation, sidebar sizing and counts,
   marked-task actions, processing feedback, and relevant shortcut-bar behavior.
6. **Search focus and responsive completion** — make query/results focus explicit, keep search
   and navigation responsive during completion, suppress duplicate operations, and refresh open
   searches after task changes. May depend on items 3–5.
7. **Completion-history reconciliation and styling** — retain tasks completed today, use
   Todoist's authoritative completion timestamps, count only remaining active tasks, and keep
   selected completed rows readable.
8. **Context-aware task creation** — inherit Today, Tomorrow, project, Inbox, and label context,
   including omitting Todoist's project field for Inbox tasks.
9. **Recoverable local Trash** — delete remotely before tombstoning locally, conditionally show
   Trash, restore tasks by recreating them, expire old tombstones, and support Empty Trash.
   Likely builds on items 3, 5, and 8.
10. **Agenda and due-time editing** — schedule incomplete Today tasks chronologically, preserve
    explicit times, suggest local times, set or clear due times, and keep long dialog input
    visible. Likely builds on items 4–6.
11. **Task Details and basic editing** — add the scrollable Task Details dialog, use `Enter` for
    details and `Space` for completion, then add focused title and description editing.
12. **Modeless and smart-view navigation** — keep the task list active, add keyboard navigation
    for projects and labels, and introduce the Today/Agenda/Tomorrow/Upcoming/Trash top bar.
13. **Persistent Projects & Labels rail** — add expanded and collapsed rail states, contextual
    task-pane titles, mouse behavior, and persisted UI preferences. Likely depends on item 12.
14. **Responsive incremental Todoist sync** — adopt Sync API tokens, apply deltas
    transactionally, recover from rejected tokens, and keep read-only UI interactions responsive
    while blocking mutations against stale state. Re-audit its relationship with items 2, 4,
    and 9 before deciding whether to stack it.
15. **AI task management** — add the review-first OpenAI proposal workflow, bounded read-only
    context tools, selectable actions, revision and confirmation flows, priority conversion, and
    partial-failure reporting. Keep this last until its task-management and UI prerequisites are
    available upstream.

## Changes

### Task Details dialog

- Added a read-only modal dialog for the selected task.
- Changed `Enter` in the task list from toggling completion to opening task details.
- Kept `Space` as the task-completion shortcut, including marked-task bulk completion.
- Displayed the full task title, description, project, status, priority, due date or time,
  deadline, duration, and recurring state.
- Added scrolling with `j`/`k`, arrow keys, Page Up/Down, Home, and End.
- Updated the shortcut bar, built-in help, and keyboard-shortcut documentation.
- Added behavior and rendering tests.

Known follow-up:

- Task labels are not shown because task-label associations are not currently loaded into
  the dialog's UI data. Add them when that relationship becomes available without an
  extra per-render database query.

Validation:

- `cargo fmt --all -- --check`
- `cargo test` (103 tests passed)

### Modeless sidebar navigation

- Kept the task list permanently active instead of switching focus between panes.
- Added unshifted `]`/`[` shortcuts to move down/up through navigation items while
  retaining `J`/`K` as alternatives.
- Routed `H`/`L` project-folder folding without requiring sidebar focus.
- Kept the navigation border passive and the task-list border active.
- Updated the shortcut bar, built-in help, README, PRD, and keyboard-shortcut
  documentation.
- Added behavior tests for bracket and uppercase sidebar navigation.

### Responsive and incremental Todoist synchronization

- Kept navigation, scrolling, search, task details, help, and quit responsive while
  post-sync data loads are in progress.
- Blocked only mutations while a potentially stale snapshot is loading, including task,
  project, and label creation, edits, completion, deletion, restoration, scheduling, and
  their confirmation dialogs.
- Preserved in-progress dialog drafts when submission is temporarily blocked.
- Replaced repeated full task-list downloads with Todoist's incremental Sync API.
- Persisted each Todoist sync token in backend settings and advanced it in the same SQLite
  transaction as the corresponding cache changes.
- Added automatic full-sync recovery when Todoist rejects a stored sync token.
- Kept locally tombstoned tasks when a remote deletion delta for the same task arrives.
- Added tests for mutation classification, sparse deletion tombstones, completion and
  recurrence conversion, incremental updates and deletions, settings preservation, and
  transactional token rollback.

Validation:

- `cargo fmt --all -- --check`
- `cargo test` (109 tests passed)

### Smart-view top navigation

- Added an always-visible top bar for Today, Agenda, Tomorrow, Upcoming, and Trash.
- Added wrapping Left/Right keyboard navigation between smart views.
- Added mouse selection for every smart-view tab.
- Kept Trash selected when it is empty instead of automatically returning to Today.
- Kept the sidebar for projects and labels, including hierarchy folding and existing
  ]/[ and J/K navigation.
- Kept arrow keys owned by open dialogs for cursor movement and dialog-specific controls.
- Updated the shortcut bar, built-in help, README, PRD, and keyboard-shortcut documentation.
- Added focused tests for keyboard cycling, mouse selection, and project/label-only sidebar
  behavior.

Validation:

- `cargo fmt --all -- --check`
- `cargo test` (113 tests passed)

### Persistent Projects & Labels rail

- Renamed the expanded sidebar to Projects & Labels.
- Changed `b` from hide/show to expanded/collapsed behavior.
- Added a three-column collapsed rail showing `b` and an expansion chevron.
- Made the collapsed rail clickable.
- Moved the bracket-navigation and collapse hints into the expanded sidebar footer.
- Muted the collapsed rail hint and task-list border to reduce visual emphasis.
- Made the task-pane title identify its selected smart view, project, or label.
- Persisted the last collapsed state and expanded width separately from user-authored
  configuration, then restored both on startup.
- Preserved `sidebar_visible` and `sidebar_width` as first-run defaults for compatibility.
- Updated configuration, shortcut, README, changelog, and built-in help documentation.

Validation:

- `cargo fmt --all -- --check`
- `cargo test` (115 tests passed)

### AI Task Management

- Added `Manage task with AI…` to Task Details and a review-first AI Task Management
  dialog backed by OpenAI.
- Let OpenAI request cached project and task context through bounded, read-only tools.
- Added proposal actions for completion notes, project and successor-task creation,
  moving the original task, and completing it.
- Split proposal review into scrollable Recommendation and Proposed Actions pages.
- Added action enable/disable controls, explicit apply confirmation, partial-failure
  reporting, and an AI revision flow that preserves the original context and proposal.
- Presented priorities as low, medium, high, and urgent while converting to Todoist API
  values only at the integration boundary.
- Added wrapped-row action scrolling, responsive footers, visible text cursors, action
  counts, progress cues, and scrollbars.
- Documented OpenAI configuration and keyboard controls.
- Advanced the development version to `0.7.0-dev.1`.

Validation:

- `cargo fmt --all -- --check`
- `cargo test`

## Upstream contribution workflow

Keep the fork's `main` as the complete personal version of Terminalist. It is the daily-use
branch and may continue receiving small changes without waiting for PR #188's work to land
upstream. Keep `codex/post-pr188` as a development or reference branch while it remains
useful.

Prepare smaller upstream contributions separately:

1. Fetch the latest `upstream/main` without replacing the fork's personal `main`.
2. Create each proposed PR branch directly from `upstream/main`.
3. Copy or reconstruct only the relevant commits and code from the personal branches.
4. Resolve upstream conflicts within that focused feature slice and validate it independently.
5. Submit the focused branch to upstream. Base it on another proposed branch only when the
   change genuinely depends on that earlier PR; otherwise keep it independently mergeable.

This keeps the installed personal version usable while PR #188 is gradually decomposed into
reviewable upstream changes. Periodically integrate new upstream changes into the personal
`main`, but do not use the personal `main` itself as the base of an upstream PR because it
contains the full integrated feature set and later work.

## Future pull request outline

Suggested title:

> Add task details dialog and post-PR #188 UI improvements

Draft summary:

> Adds a read-only, scrollable details dialog so task titles and metadata are available
> without editing the task. `Enter` opens details and `Space` remains the completion
> shortcut.

Before opening a pull request:

1. Rebase onto the final PR #188 merge result or the latest intended base.
2. Review this file against the actual branch diff and remove stale notes.
3. Run the required formatting and test commands again.
4. Replace the validation counts above with the final results.
5. Decide whether the accumulated changes form one coherent PR or should be split.
